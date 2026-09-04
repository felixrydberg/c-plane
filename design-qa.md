# Design QA

- Source visual truth: `C:/Users/balle/.codex/generated_images/01a06915-e2e6-7161-b2d0-6fb2fa39cf4b/exec-3766c679-c48d-4e09-81ff-a90c60f2aa0d.png`
- Implementation screenshot: `C:/Users/balle/.codex/visualizations/2026/09/04/01a06d06-4a26-7963-8d74-2c62a88200b0/environment-review-implementation.png`
- Route: `http://localhost:3000/acme/containers/ed414971-fe14-41c3-a68d-840866d90132/e08ffa17-5464-4759-9807-e58e072c1e74`
- Viewport: 1487 × 1058 CSS px
- Pixel dimensions: source 1487 × 1058; implementation 1487 × 1058
- Density normalization: 1:1 pixel comparison; no scaling or device frame normalization required
- State: dark theme, `main` default environment, one reversible pending container replica change, release-diff modal open

## Full-view comparison

The source and implementation were opened together in one comparison input at the same viewport. The source established the dark CPlane visual language, environment context, change summary, and deployed-resource table. The implementation now follows the updated product direction: the inline change list is replaced by a compact alert that opens a focused differences modal, while deployed containers use the established table design.

The source mock contains three example changes and a Postgres branch; the rendered project data contains one changed container and no linked database resource. That content difference is expected and does not change the layout decision. The modal treatment is an intentional product change requested after the original mock, not an accidental fidelity drift.

Required fidelity surfaces:

- Fonts and typography: the existing CPlane font stack, weights, compact labels, and hierarchy remain consistent with the source. Modal copy is short enough to avoid awkward wrapping.
- Spacing and layout rhythm: the established sidebar, page gutters, section gaps, table density, alert treatment, modal padding, and footer action spacing are balanced at the target viewport. A 1024 × 768 breakpoint check kept the table and primary actions usable.
- Colors and tokens: dark surfaces, neutral borders, warning alert state, success/live state, and primary Add/Deploy actions use the existing product tokens.
- Image quality and assets: there are no photographic or generated image assets in this flow. Product icons use the existing icon system; no placeholder, CSS-drawn, or handcrafted SVG assets were introduced.
- Copy and content: the page now says `Containers`, `Currently deployed`, `pending change`, `Review changes`, and `Live → Pending`, removing the draft/deployed jargon and the Postgres branch list that the user called out.

## Focused region comparison

The modal was inspected as the focused region because it is the new core interaction. Its change count, live/pending values, edit affordance, and footer actions are legible and aligned. The deployed table was also inspected for the original Service, Image, Replicas, Port, Access, and Updated columns.

## Findings

- No actionable P0, P1, or P2 visual differences remain.
- No accessibility blocker was observed in the reviewed flow: the alert exposes a named button action, the modal has a text Close action (no X close control), the disclosure reports expanded state, and disabled/loading states are conveyed.

## Open Questions

- The current account has no linked Postgres branch, so the removed branch-list behavior could not be exercised with live data. The page no longer fetches or renders that section by design.
- A transient Vue `parentNode` error appeared during an earlier HMR interaction, but it did not recur after the UI container restart and a clean page reload; no page-load console error remains in the final state.

## Primary interactions tested

- Opened the pending-change alert action and verified the differences modal.
- Verified the modal shows live-versus-pending values and keeps Deploy/Discard actions inside the review context.
- Verified the main page contains no Discard changes or Deploy changes buttons.
- Verified Add container is the single primary page action.
- Verified the deployed list has the same search-and-refresh toolbar pattern as Object Storage, Registry, and Postgres.
- Verified searching by service/image returns matching rows, shows a no-match state, and clearing search restores the list.
- Verified the deployed section renders the established table columns and links to the live container version.
- Created and discarded a temporary replica change to leave the project clean; no deployment was performed.

## Comparison History

- Pass 1: the initial inline review layout was compared against the source mock at a matched 1487 × 1058 viewport.
- Pass 2: applied the user-requested modal-driven review, compact alert, table restoration, Postgres section removal, and action relocation; recaptured the same viewport and compared source plus implementation together.
- Pass 3: removed the default modal X control to follow the button style guide, recaptured the modal, and confirmed no new page-load console errors after reload.
- Pass 4: removed the nested button role from the clickable alert, recaptured the modal, and confirmed the final DOM has no nested interactive alert control.
- Pass 5: added the shared deployed-container search field with Refresh beside it and verified matching, no-match, and cleared-search states in the Docker-served preview.

## Implementation Checklist

- [x] Restore the established deployed-container table.
- [x] Replace the large inline change section with a compact clickable alert and differences modal.
- [x] Remove the Postgres compute-branch list from this page.
- [x] Keep Deploy and Discard actions available inside the review modal, not in the page header.
- [x] Make Add container the single primary page action.
- [x] Match the shared search-and-refresh toolbar pattern.
- [x] Leave the local project data clean after reversible QA interactions.

## Follow-up Polish

- [P3] Validate the modal with a release containing several containers and changed fields once representative fixture data exists.

final result: passed
