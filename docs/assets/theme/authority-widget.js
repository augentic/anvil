(function () {
  "use strict";

  function initAuthorityWidget() {
    var ctl = document.getElementById("auth-ctl");
    var flow = document.getElementById("auth-flow");
    if (!ctl || !flow) return;

    var steps = flow.querySelectorAll(".auth-step");

    function paint(active) {
      steps.forEach(function (s) {
        s.classList.remove("active");
        s.classList.remove("skipped");
        var n = parseInt(s.getAttribute("data-step"), 10);
        if (n < active) {
          s.classList.add("skipped");
        } else if (n === active) {
          s.classList.add("active");
        }
      });
    }

    var scenarios = { slice: 1, evidence: 2, default: 3, tied: 4 };

    ctl.querySelectorAll("button").forEach(function (b) {
      b.addEventListener("click", function () {
        ctl.querySelectorAll("button").forEach(function (x) {
          x.classList.remove("on");
        });
        b.classList.add("on");
        var s = b.getAttribute("data-scenario");
        paint(scenarios[s] || 1);
      });
    });

    paint(1);

    steps.forEach(function (s) {
      s.addEventListener("click", function () {
        var n = parseInt(s.getAttribute("data-step"), 10);
        ctl.querySelectorAll("button").forEach(function (x) {
          x.classList.remove("on");
        });
        var keys = Object.keys(scenarios);
        for (var i = 0; i < keys.length; i++) {
          if (scenarios[keys[i]] === n) {
            var btn = ctl.querySelector(
              'button[data-scenario="' + keys[i] + '"]',
            );
            if (btn) btn.classList.add("on");
          }
        }
        paint(n);
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initAuthorityWidget);
  } else {
    initAuthorityWidget();
  }
})();
