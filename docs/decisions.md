# Decision log

Product and scope decisions for BW Darkroom, newest first. Model decisions (D1–D11) are in
[design 0001](design/0001-negative-simulation-model.md#0-summary-of-decisions).

| # | Date | Decision | Rationale |
|---|------|----------|-----------|
| P1 | 2026-10-23 | **The digital look is deferred. Work concentrates on the core use case: film, chemicals and paper.** The digital look is a separate output that would map paper Dmax to screen black, offer contrast beyond paper grades, expose night scenes automatically and take the starting brightness from the camera. It stays planned but unscheduled. Until it is revisited, new work goes to the film, developer, negative and print models and the controls for them. | The physically grounded darkroom pipeline is what sets the app apart, and it still has gaps: film and developer parameters are illustrative, and the Tri-X + D-76 fit is not done. A digital look would be a deliberate departure from that model, so it should stay separate and be labelled as such when it comes. |
