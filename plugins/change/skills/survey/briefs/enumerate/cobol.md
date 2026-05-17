---
id: enumerate
description: Per-language surface enumeration prompt for /change:survey (COBOL).
language: cobol
---

# COBOL surface enumeration for `/change:survey`

This brief drives the LLM that produces a candidate `surfaces.json` for a single `legacy-code` source whose primary language is COBOL. The skill resolves this file by language and feeds it to the LLM; the validated, canonicalized output is consumed by the rest of `/change:survey`.

COBOL is the most fragile language in the v1 brief set. Mainframe codebases mix dialects (IBM Enterprise COBOL, Micro Focus, GnuCOBOL), embed framework calls (`EXEC CICS`, `EXEC SQL`, `MQPUT`) the LLM has to recognize from syntax alone, and reach surfaces (JCL, scheduler config) that often live outside the source root entirely. The bounded repair loop will catch most shape errors, but operators should still review COBOL candidates more closely than TypeScript or Rust ones and expect to edit candidates by hand more often than for the other languages.

## Scope

Frameworks covered in v1:

- **CICS** — BMS map-driven transactions, `EXEC CICS RECEIVE` / `EXEC CICS WEB RECEIVE` request handlers, `EXEC CICS PUT QUEUE` writers.
- **IMS DC** — message processing programs (MPPs) reading from `IO-PCB` via `GU` / `GN`.
- **MQ Series** — `MQPUT` producers, `MQGET` consumers, and programs invoked via MQ trigger monitors.
- **Batch JCL** — `EXEC PGM=` job steps in the source root, including steps wired to enterprise schedulers (CA-7, Control-M, Tivoli Workload Scheduler) when scheduler-trigger evidence is visible inline.

**Copybook flattening is a precondition.** Before the brief runs, copybooks `COPY`'d into a program are folded into that program's `touches[]` set. The LLM is not expected to reconstruct copybook boundaries from a partial source root — every copybook referenced in scope must exist on disk under the source root for it to land in `touches[]`.

COBOL enumeration is **best-effort**. Dialect quirks, partial source trees (the legacy-code source rarely contains every JCL deck or every copybook), and the mainframe library boundary mean candidates frequently need operator review before reaching `propose`.

## Schema

Output is a JSON document matching the `surfaces.json` schema. Top-level fields: `version` (integer, must be `1`), `source-key` (kebab-case string), `language` (use `cobol`), and `surfaces` (array of `Surface` objects sorted by `id`).

Every `Surface` object MUST contain exactly these fields (no extras):

| Field         | Type                | Notes                                                                                                  |
| ------------- | ------------------- | ------------------------------------------------------------------------------------------------------ |
| `id`          | string, non-empty   | Stable identifier unique within this file. Reruns diff cleanly.                                        |
| `kind`        | string, closed enum | One of `http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`. |
| `identifier`  | string, non-empty   | Legacy spelling of the observable surface (CICS transaction id, MQ queue name, JCL job/step, etc.).    |
| `handler`     | string, non-empty   | Handler reference, typically `<program>.cbl:<paragraph>`.                                              |
| `touches`     | string[]            | Source files reached from the handler, sorted alphabetically, relative to the source root.            |
| `declared-at` | string[], min 1     | Declaration sites where the surface is registered with its framework, sorted alphabetically; relative paths optionally `:<line>` suffixed. Non-empty. |

**Path-under-source-root rule.** Every entry in `touches[]` and `declared-at[]` MUST be a relative path with no leading `/`, no Windows drive letter, and no `..` segments. Joined with the source root the path MUST resolve to a file inside the source root. Mainframe-side libraries are out of scope: copybooks shipped under the source root (e.g. `COPYLIB/CUSTREC.cpy`) are in scope; mainframe partitioned datasets such as `SYS1.MACLIB`, `CEE.SCEELKED`, or `SYS2.PROCLIB` are not — never emit them in `touches[]` or `declared-at[]`, even when a `COPY` directive or JCL `STEPLIB` references them.

The CLI canonicalizes the document before write: `surfaces[]` is sorted by `id`; `touches[]` and `declared-at[]` are sorted alphabetically. The agent's emission order does not influence the canonical form, but emitting in canonical order keeps the repair loop's diff comprehensible.

## Worked examples

Each example shows the input snippet, the framework signature that fired, and the expected `Surface` block in canonical form.

### `http-route` — CICS Web Services SOAP handler

Pure-CICS shops without an HTTP edge often have **zero** `http-route` surfaces; only emit when the source contains `EXEC CICS WEB` calls or a 3270-to-HTTP gateway. Skip the kind otherwise rather than retro-fitting `http-route` onto plain BMS screens.

Input (`PAYROLL/PAY010.cbl`):

```cobol
       IDENTIFICATION DIVISION.
       PROGRAM-ID. PAY010.
       PROCEDURE DIVISION.
       1000-MAIN.
           EXEC CICS WEB RECEIVE
               MEDIATYPE('application/soap+xml')
               INTO(WS-REQUEST)
           END-EXEC.
           PERFORM 2000-PROCESS-PAYROLL.
           EXEC CICS WEB SEND
               FROM(WS-RESPONSE)
           END-EXEC.
```

Signature: `EXEC CICS WEB RECEIVE` in `1000-MAIN` ⇒ HTTP-style surface fronted by CICS Web Services. Transaction id `PAYW` declared in the matching CICS resource definition (`CSD/PAYROLL.csd:42`).

```json
{
  "id": "http-cics-web-payw",
  "kind": "http-route",
  "identifier": "PAYW",
  "handler": "PAYROLL/PAY010.cbl:1000-MAIN",
  "touches": [
    "COPYLIB/PAYREC.cpy",
    "PAYROLL/PAY010.cbl",
    "PAYROLL/PAY020.cbl"
  ],
  "declared-at": ["CSD/PAYROLL.csd:42"]
}
```

### `message-pub` — `EXEC CICS PUT QUEUE` writer

Input (`PAYROLL/PAY020.cbl`):

```cobol
       3000-PUBLISH-AUDIT.
           EXEC CICS PUT QUEUE
               QUEUE('AUDITLOG')
               FROM(WS-AUDIT-RECORD)
           END-EXEC.
```

Signature: `EXEC CICS PUT QUEUE` ⇒ `message-pub` keyed on the TSQ name.

```json
{
  "id": "message-pub-auditlog",
  "kind": "message-pub",
  "identifier": "AUDITLOG",
  "handler": "PAYROLL/PAY020.cbl:3000-PUBLISH-AUDIT",
  "touches": [
    "COPYLIB/AUDITREC.cpy",
    "PAYROLL/PAY020.cbl"
  ],
  "declared-at": ["PAYROLL/PAY020.cbl:128"]
}
```

`MQPUT` to MQ Series follows the same shape: handler is the paragraph containing the `MQPUT` call; `identifier` is the queue name resolved from the `MQOD-OBJECTNAME` field; `declared-at` is the `MQPUT` call site.

### `message-sub` — `MQGET` consumer / triggered program

Input (`BILLING/BILL050.cbl`):

```cobol
       IDENTIFICATION DIVISION.
       PROGRAM-ID. BILL050.
       PROCEDURE DIVISION.
       0000-MAIN.
           CALL 'MQOPEN' USING HCONN, MQOD, MQOO-INPUT, HOBJ, COMPCODE, REASON.
           PERFORM 2000-CONSUME-LOOP UNTIL END-OF-QUEUE.
       2000-CONSUME-LOOP.
           CALL 'MQGET' USING HCONN, HOBJ, MQMD, MQGMO,
                               BUFFER-LENGTH, BUFFER, DATA-LENGTH,
                               COMPCODE, REASON.
           PERFORM 3000-APPLY-PAYMENT.
```

Signature: `MQGET` inside a per-message paragraph ⇒ `message-sub` keyed on the queue (resolved from the matching `MQOPEN` / `MQOD-OBJECTNAME` constant, here `PAYMENT.IN`). Triggered programs invoked via MQ trigger monitors take the same shape: the entry program is the handler and the queue name comes from the trigger definition referenced inline.

```json
{
  "id": "message-sub-payment-in",
  "kind": "message-sub",
  "identifier": "PAYMENT.IN",
  "handler": "BILLING/BILL050.cbl:2000-CONSUME-LOOP",
  "touches": [
    "BILLING/BILL050.cbl",
    "BILLING/BILL060.cbl",
    "COPYLIB/PAYMTREC.cpy"
  ],
  "declared-at": ["BILLING/BILL050.cbl:78"]
}
```

### `scheduled-job` — JCL job step on a scheduler

Input (`JCL/PAYDAILY.jcl`):

```jcl
//PAYDAILY JOB (ACCT),'PAYROLL DAILY',CLASS=A,MSGCLASS=H
//*  CA-7 SCHEDULE: PAYDAILY  TRIGGERED BY DSN=PROD.PAYROLL.READY
//STEP01   EXEC PGM=PAY100,PARM='RUN-MODE=PROD'
//STEPLIB  DD DSN=PAYROLL.LOADLIB,DISP=SHR
//SYSIN    DD DSN=PAYROLL.PARMS(PAYDAILY),DISP=SHR
```

Signature: a JCL job containing `EXEC PGM=<NAME>` with a scheduler-trigger comment (`CA-7 SCHEDULE:`, `CONTROL-M:`, `TWS:`) ⇒ `scheduled-job` keyed on the JCL job/step. The scheduler config itself typically lives outside the source root; the brief enumerates the program entry point and notes the scheduler context in `declared-at[]` only when visible inline.

```json
{
  "id": "scheduled-job-paydaily",
  "kind": "scheduled-job",
  "identifier": "PAYDAILY.STEP01",
  "handler": "PAYROLL/PAY100.cbl:0000-MAIN",
  "touches": [
    "COPYLIB/PAYREC.cpy",
    "PAYROLL/PAY100.cbl",
    "PAYROLL/PAY110.cbl"
  ],
  "declared-at": ["JCL/PAYDAILY.jcl:3"]
}
```

### `cli-command` — JCL `EXEC PGM=` with `PARM=` arguments

The "CLI" framing is a stretch on the mainframe — there is no shell — but a COBOL program invoked by a JCL step with `PARM=` passing arguments is the closest analogue and is what `propose` will see as a callable entry point. Only emit `cli-command` for steps that are *not* already covered by `scheduled-job`; treat the scheduled / ad-hoc distinction as best-effort and prefer `scheduled-job` when in doubt.

Input (`JCL/PAYREPRT.jcl`):

```jcl
//PAYREPRT JOB (ACCT),'AD-HOC REPORT',CLASS=A
//STEP01   EXEC PGM=PAY200,PARM='REGION=EMEA,FORMAT=CSV'
```

```json
{
  "id": "cli-command-payreprt",
  "kind": "cli-command",
  "identifier": "PAYREPRT.STEP01",
  "handler": "PAYROLL/PAY200.cbl:0000-MAIN",
  "touches": [
    "PAYROLL/PAY200.cbl",
    "PAYROLL/PAY210.cbl"
  ],
  "declared-at": ["JCL/PAYREPRT.jcl:2"]
}
```

### `external-call-out` — `CALL` to external program or DB2 stored proc

Input (`PAYROLL/PAY100.cbl`):

```cobol
       4000-FETCH-RATE.
           CALL 'TAXRATE' USING WS-COUNTRY, WS-RATE.

       4100-POST-LEDGER.
           EXEC SQL
               CALL ACCTNG.POST_LEDGER(:WS-BATCH-ID, :WS-AMOUNT)
           END-EXEC.
```

Signature: static `CALL 'PROGNAME'` to a program outside the current source unit, or `EXEC SQL CALL <SCHEMA>.<PROC>` to a DB2 stored procedure ⇒ one `external-call-out` per distinct callee. Dynamic `CALL identifier` is in scope only when the identifier is a hard-coded `VALUE` constant the brief can resolve statically; otherwise skip it (see anti-patterns).

```json
{
  "id": "external-call-acctng-post-ledger",
  "kind": "external-call-out",
  "identifier": "ACCTNG.POST_LEDGER",
  "handler": "PAYROLL/PAY100.cbl:4100-POST-LEDGER",
  "touches": ["PAYROLL/PAY100.cbl"],
  "declared-at": ["PAYROLL/PAY100.cbl:412"]
}
```

```json
{
  "id": "external-call-taxrate",
  "kind": "external-call-out",
  "identifier": "TAXRATE",
  "handler": "PAYROLL/PAY100.cbl:4000-FETCH-RATE",
  "touches": ["PAYROLL/PAY100.cbl"],
  "declared-at": ["PAYROLL/PAY100.cbl:405"]
}
```

## `handler` resolution

The handler is the `PROGRAM-ID` plus the entry paragraph for the surface, formatted as `<relative-program-path>:<paragraph-name>` (e.g. `PAYROLL/PAY001.cbl:0000-MAIN`).

- **CICS surfaces.** The paragraph reached immediately after the `EXEC CICS RECEIVE` / `EXEC CICS WEB RECEIVE` that initiates the transaction (typically `0000-MAIN` or the first `PERFORM` target).
- **IMS DC surfaces.** The paragraph wrapping the `GU` / `GN` against `IO-PCB`.
- **MQ surfaces.** The paragraph that contains the `MQPUT` (for `message-pub`) or `MQGET` (for `message-sub`). For triggered programs invoked by an MQ trigger monitor, use the program's entry paragraph (`0000-MAIN` or equivalent).
- **JCL surfaces (`scheduled-job`, `cli-command`).** The program named in `EXEC PGM=<NAME>` is the handler; resolve `<NAME>` to its `.cbl` file under the source root and pair it with the program's entry paragraph (e.g. `PAYROLL/PAY100.cbl:0000-MAIN`). The JCL job/step name is the `identifier`, not the handler — never collapse the two.
- **`external-call-out` surfaces.** The paragraph containing the `CALL` statement or `EXEC SQL CALL`. The callee is captured in `identifier`, not in `handler`.

## `touches[]` resolution

Start at the handler's program file and walk two graphs, bounded by the source root:

1. **Copybook fan-in.** Every `COPY COPYBOOK-NAME` directive in the handler program resolves to a copybook file under the source root (commonly `COPYLIB/<NAME>.cpy`); each resolved copybook joins the `touches[]` set. After copybook flattening, every copybook the program transitively `COPY`s is included.
2. **Called-program graph.** Static `CALL "PROGNAME"` statements add the callee program file to `touches[]`. Dynamic `CALL identifier` is resolvable only when the identifier is bound to a hard-coded `VALUE` constant in `WORKING-STORAGE` that the brief can read off the source — when the constant is not visible, drop the edge (the candidate algorithm can still partition without it).

Stop at the source-root boundary in both walks. Do NOT chase mainframe-side libraries (`SYS1.MACLIB`, `CEE.SCEELKED`, `SYS2.PROCLIB`) even when the LLM "knows" what they probably contain. If a `COPY` directive or `CALL` target points outside the source root, drop the edge silently — the resulting `touches[]` may be incomplete, but it stays inside the contract.

JCL decks reachable from the source root that drive the handler program (`scheduled-job`, `cli-command`) belong in `declared-at[]`, not in `touches[]`.

## Anti-patterns

Do NOT emit:

- **Mainframe library paths** — `SYS1.MACLIB`, `CEE.SCEELKED`, `SYS2.PROCLIB`, or any other partitioned-dataset reference — in `touches[]` or `declared-at[]`. Drop the edge instead.
- **Copybooks that resolve outside the source root.** A `COPY CUSTREC` directive only contributes to `touches[]` when `COPYLIB/CUSTREC.cpy` (or equivalent) actually exists in the source tree.
- **Dynamic `CALL` targets** the brief cannot resolve statically. Skip the edge rather than guessing.
- **Test JCL or compile-only JCL job steps.** Job names matching `*TEST*` (case-insensitive) or steps named `STEP-COMPILE` / `STEPCOMP` / `COMPILE` only — exclude both from `scheduled-job` and `cli-command` enumeration.
- **Absolute paths** (leading `/`, Windows drive letters) or `..` traversal in any path field.
- **Hallucinated CICS / MQ / IMS calls.** If the program does not contain `EXEC CICS`, `MQPUT` / `MQGET`, or `IO-PCB` reads, there are no CICS / MQ / IMS surfaces in that program — do not invent them from naming conventions.
- **`PROGRAM-ID` conflated with the JCL job name.** For `scheduled-job` and `cli-command`, the `identifier` is the JCL `<JOB>.<STEP>` (or `<JOB>`) and the `handler` is the program named in `EXEC PGM=`; the two are never the same string.
- **Duplicate `id` values within one `surfaces.json`.** The CLI rejects collisions; pick distinct stable ids (e.g. `scheduled-job-paydaily` vs `scheduled-job-paydaily-step02`) when one program backs more than one JCL step.

The bounded repair loop in the survey skill will surface validator errors back into a re-prompt, but the cheapest fix is to avoid the anti-patterns above on the first pass.
