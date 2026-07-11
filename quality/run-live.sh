#!/bin/bash
# Run the canonical full-loop live profile and write a structured evidence bundle.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
scenario="${SCENARIO:-guest-execute-loop}"
profile="${1:-native-live}"
case "$profile" in
  native-live) shim=native ;;
  wasm-live) shim=guest ;;
  *) echo "profile must be native-live or wasm-live" >&2; exit 2 ;;
esac

command -v cursor-agent >/dev/null || { echo "cursor-agent not found" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq not found" >&2; exit 2; }

scenario_file="$root/quality/scenarios/$scenario.yaml"
test -f "$scenario_file" || { echo "unknown scenario: $scenario" >&2; exit 2; }
trials="${TRIALS:-$(awk -v id="$profile" '
  $0 == "- id: " id { found=1; next }
  found && $1 == "trials:" { print $2; exit }
' "$scenario_file")}"
test -n "$trials" || { echo "profile $profile is not declared by $scenario" >&2; exit 2; }

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
run_id="$scenario-$profile-$stamp"
bundle="${RUN_BUNDLE:-$root/quality/runs/$run_id}"
mkdir -p "$bundle/trials"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
specify_revision="$(git -C "$root" rev-parse HEAD)"
adapters_root="${SPECIFY_ADAPTERS:-$root/../specify-adapters}"
adapters_revision="$(git -C "$adapters_root" rev-parse HEAD)"
prompt_digest="sha256:$(shasum -a 256 "$scenario_file" "$root/quality/rubrics/semantic.yaml" | shasum -a 256 | awk '{print $1}')"

component_json='{}'
if test "$shim" = guest; then
  component_json="$(
    for component in "$adapters_root"/target/wasm32-wasip2/release/*.wasm; do
      test -f "$component" || continue
      printf '%s\tsha256:%s\n' "$(basename "$component" .wasm)" "$(shasum -a 256 "$component" | awk '{print $1}')"
    done | jq -Rn '[inputs | split("\t")] | map({(.[0]): .[1]}) | add // {}'
  )"
fi

trial_files=()
overall=pass
for trial in $(seq 1 "$trials"); do
  trial_root="$bundle/trials/$trial"
  mkdir -p "$trial_root"
  log="$trial_root/driver.log"
  trial_started="$(date +%s)"
  if ! SPECIFY_SHIM="$shim" SPECIFY_SANDBOX="$trial_root" \
      bash "$root/quality/profiles/guest-execute-loop.sh" >"$log" 2>&1; then
    driver_outcome=fail
  else
    driver_outcome=pass
  fi
  workspace="$trial_root/guest-execute-loop"
  test "$shim" = native && workspace="$workspace-native"

  hard='[]'
  hard_pass=true
  add_hard() {
    local id="$1" passed="$2" evidence="$3" detail="${4:-}"
    local outcome=pass
    test "$passed" = true || { outcome=fail; hard_pass=false; }
    hard="$(jq -c --arg id "$id" --arg outcome "$outcome" --arg evidence "$evidence" \
      --arg detail "$detail" '. + [{id:$id,outcome:$outcome,evidence:$evidence,detail:(if $detail=="" then null else $detail end)}]' <<<"$hard")"
  }

  drained=false
  if test "$driver_outcome" = pass && test -f "$workspace/plan.yaml" \
      && ! grep -Eq 'status: (pending|in-progress)' "$workspace/plan.yaml"; then drained=true; fi
  add_hard guest-loop-drained "$drained" plan.yaml

  journal_ok=false
  if test -f "$workspace/.specify/journal.jsonl" \
      && grep -q '"slice.merge.succeeded"' "$workspace/.specify/journal.jsonl" \
      && grep -q '"slice.archive.created"' "$workspace/.specify/journal.jsonl"; then journal_ok=true; fi
  add_hard guest-journal-cadence "$journal_ok" .specify/journal.jsonl

  generated_ok=true
  generated_count=0
  for manifest in "$workspace"/crates/*/Cargo.toml; do
    test -f "$manifest" || continue
    generated_count=$((generated_count + 1))
    cargo check --manifest-path "$manifest" >>"$log" 2>&1 || generated_ok=false
  done
  test "$generated_count" -gt 0 || generated_ok=false
  add_hard guest-generated-crate-verifies "$generated_ok" crates/

  marker_ok=false
  test ! -e "$workspace/.specify/guest.lock" && marker_ok=true
  add_hard guest-marker-released "$marker_ok" .specify/guest.lock

  rubric_prompt="Read the generated baseline specifications and plan in this workspace. Grade only the guest-spec-sensible criterion in $root/quality/rubrics/semantic.yaml. Return exactly one compact JSON object with keys score (integer 0-100), outcome (pass or fail; pass requires score >= 80), and detail (concise evidence-based explanation)."
  if rubric_raw="$(cd "$workspace" && cursor-agent --print "$rubric_prompt" 2>>"$log")" \
      && rubric="$(jq -ce 'select((.score|type)=="number" and (.outcome=="pass" or .outcome=="fail") and (.detail|type)=="string")' <<<"$rubric_raw")"; then
    :
  else
    rubric='{"score":0,"outcome":"error","detail":"semantic grader did not return valid JSON"}'
  fi
  printf '%s\n' "$rubric" >"$trial_root/rubric.json"
  rubric_outcome="$(jq -r .outcome <<<"$rubric")"
  test "$rubric_outcome" = pass || overall=fail
  $hard_pass || overall=fail

  duration_ms="$(( ($(date +%s) - trial_started) * 1000 ))"
  trial_file="$trial_root/result.json"
  jq -n \
    --argjson trial "$trial" --arg profile "$profile" \
    --arg outcome "$(if $hard_pass && test "$rubric_outcome" = pass; then echo pass; else echo fail; fi)" \
    --argjson hard "$hard" --argjson rubric "$rubric" --argjson duration "$duration_ms" \
    '{trial:$trial,profile:$profile,outcome:$outcome,"hard-assertions":$hard,"semantic-rubrics":[{id:"guest-spec-sensible",outcome:$rubric.outcome,score:$rubric.score,evidence:"rubric.json",detail:$rubric.detail}],metrics:{"usage-available":false,"input-tokens":0,"output-tokens":0,"reasoning-tokens":0,"duration-ms":$duration},outputs:["driver.log","rubric.json"]}' >"$trial_file"
  trial_files+=("$trial_file")
done

completed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
trials_json="$(jq -s . "${trial_files[@]}")"
jq -n \
  --arg scenario "$scenario" --arg outcome "$overall" --arg id "$run_id" \
  --arg profile "$profile" --arg model "${SPECIFY_EVAL_MODEL:-cursor-default}" \
  --arg specify_revision "$specify_revision" --arg adapters_revision "$adapters_revision" \
  --arg prompt_digest "$prompt_digest" --arg started "$started_at" --arg completed "$completed_at" \
  --argjson components "$component_json" --argjson trials "$trials_json" \
  '{version:1,scenario:$scenario,outcome:$outcome,run:{id:$id,runner:("quality/run-live.sh "+$profile),revisions:{specify:$specify_revision,"specify-adapters":$adapters_revision},model:$model,"prompt-digest":$prompt_digest,"component-digests":$components,"started-at":$started,"completed-at":$completed},trials:$trials}' >"$bundle/report.json"

echo "$bundle/report.json"
test "$overall" = pass
