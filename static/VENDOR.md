## htmx

`htmx.min.js` - vendored from https://unpkg.com/htmx.org@2.0.4/dist/htmx.min.js, version 2.0.4, BSD 2-Clause license.

Vendored (not loaded from a CDN) to keep the self-hosted single-binary story intact and no external runtime dependency.

## favicon.svg

`favicon.svg` - the "feather" icon from [Feather Icons](https://feathericons.com) (github.com/feathericons/feather), MIT license. `stroke` hardcoded to the app's accent color (`#51459e`) — the original uses `stroke="currentColor"`, which resolves to plain black when used standalone as a favicon (no page CSS context to inherit from).


## GitHub mark (inlined in templates/admin_login.html)

The GitHub sign-in button's icon is the "Invertocat" mark from the [GitHub Brand Toolkit](https://brand.github.com/foundations/logo), inlined directly as SVG markup in `templates/admin_login.html` (not a separate `static/` file). `fill` set to `currentColor` — unlike the favicon (rendered standalone in browser chrome, with no page CSS to inherit from), this SVG sits inside a styled button as real page content, so it correctly tracks the button's own `color`, including on `:hover` when the button inverts to a solid fill. The originally-downloaded asset is black-only.
