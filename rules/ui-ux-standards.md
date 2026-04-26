# UI / UX Standards

> Every CSS/layout change must satisfy these rules.  
> Violations block PR merge.  
> Last updated: 2026-04-25

---

## 1. Layout System

### 1.1 No `position: fixed/absolute` for layout elements

Layout elements (nav bars, mode toggles, badges, toolbars) **must** participate
in the document flow via `flex` or `grid`. Only the following may use
`position: fixed`:

| Allowed fixed-position use | Example |
|---|---|
| Truly floating overlays (modals, toasts, context menus) | `QuestRewardCeremony`, `ComboToast`, `PetContextMenu` |
| Mobile bottom nav (OS-level chrome equivalent) | `.mobile-bottom-nav` |
| Full-screen backdrops | `SplashScreen`, `SkillConstellation` overlay |

Everything else belongs inside a flex/grid container.

### 1.2 Containers group related elements

Buttons, labels, and badges that belong together must be wrapped in a semantic
container (`<div>`, `<nav>`, `<header>`) and laid out with `flex`/`grid` — never
individually positioned with absolute/fixed offsets.

```vue
<!-- ✅ Good: toolbar as a flex row -->
<header class="chat-toolbar">
  <ModeToggle />
  <span class="spacer" />
  <StatusBadge />
</header>

<!-- ❌ Bad: each element fixed-positioned independently -->
<div class="mode-toggle" style="position: fixed; top: 12px; left: 82px" />
<div class="status-badge" style="position: fixed; top: 12px; right: 16px" />
```

### 1.3 No magic pixel offsets

Never hardcode pixel values that depend on the size of a sibling element (e.g.
`left: 82px` because the sidebar is 72px + 10px gap). Use flex/grid gaps,
padding, or CSS variables instead.

```css
/* ❌ Bad: brittle coupling to sidebar width */
.mode-pill { position: fixed; left: 82px; }

/* ✅ Good: pill sits in the layout flow */
.app-header { display: flex; gap: var(--ts-space-sm); }
```

---

## 2. Z-Index Scale

Use the `--ts-z-*` design tokens defined in `src/style.css`. Never use bare
integer z-index values.

| Token | Value | Use for |
|---|---|---|
| `--ts-z-base` | `1` | In-flow elements needing stacking context |
| `--ts-z-dropdown` | `100` | Dropdowns, popovers, tooltips |
| `--ts-z-sticky` | `200` | Sticky headers, floating toolbars |
| `--ts-z-dialog` | `300` | Modal dialogs, wizard overlays |
| `--ts-z-overlay` | `500` | Full-screen overlays (reward ceremony, constellation) |
| `--ts-z-toast` | `700` | Toast notifications |
| `--ts-z-splash` | `900` | Splash screen, loading screens |
| `--ts-z-context-menu` | `1000` | Right-click context menus (must beat everything) |

```css
/* ✅ Good */
.my-dialog { z-index: var(--ts-z-dialog); }

/* ❌ Bad */
.my-dialog { z-index: 9999; }
```

---

## 3. Responsive Breakpoints

### 3.1 Standard breakpoints

Use the `--ts-bp-*` tokens (defined in `src/style.css`) in all media queries:

| Token | Value | Meaning |
|---|---|---|
| `--ts-bp-mobile` | `640px` | Mobile ↔ desktop threshold |
| `--ts-bp-tablet` | `840px` | Compact ↔ comfortable desktop |

CSS custom properties cannot appear inside `@media` conditions, so reference
the **raw value** from the token table above. The tokens exist as documentation
anchors and for use in JavaScript (`getComputedStyle`).

```css
/* ✅ Correct — uses the standard 640px breakpoint */
@media (max-width: 640px) { ... }

/* ❌ Wrong — ad-hoc breakpoint */
@media (max-width: 720px) { ... }
```

### 3.2 Mobile-first defaults

Prefer `min-width` (additive) over `max-width` (subtractive) when practical.
The existing codebase uses `max-width: 640px`; new code should follow the same
pattern for consistency until a full migration is done.

---

## 4. Design Tokens

### 4.1 Colors must use tokens

Every color value must come from a `var(--ts-*)` token. No bare hex, `rgb()`,
or `hsl()` values.

```css
/* ✅ Good */
color: var(--ts-text-primary);
background: var(--ts-bg-surface);

/* ❌ Bad */
color: #f1f5f9;
background: #1e293b;
```

If a design need isn't covered by existing tokens, add a new token to
`src/style.css` under the appropriate section — don't inline the color.

### 4.2 Spacing must use tokens

Use `var(--ts-space-*)` for padding, margin, and gap. Exceptions: `1px`
borders, `0`, and `100%` are fine as literals.

### 4.3 Border-radius must use tokens

Use `var(--ts-radius-*)`. Never hardcode `border-radius: 8px`.

---

## 5. Component Patterns

### 5.1 Scoped styles

All Vue component `<style>` blocks must use `scoped` to prevent class name
collisions. Only `App.vue` and `style.css` may define global styles.

### 5.2 No `!important`

`!important` indicates a specificity war. Fix the root cause instead:
- Use a more specific selector.
- Move the override into the component's scoped style.
- Use a CSS class toggle instead of an override.

### 5.3 No hardcoded inline styles in templates

Use class bindings (`:class`) or CSS custom properties (`:style="{ '--x': val }"`)
instead of `style="display:flex;gap:8px"`. Dynamic values for computed positions
(e.g. draggable element `transform`) are an acceptable exception.

### 5.4 Transitions use tokens

Use `var(--ts-transition-fast)`, `--ts-transition-normal`, or
`--ts-transition-slow`. Don't hardcode `transition: 0.2s ease`.

---

## 6. Accessibility Baseline

- Interactive elements (`<button>`, `<a>`, custom controls) must have a
  `title`, `aria-label`, or visible text.
- Color must not be the **only** indicator of state — pair with icons, text,
  or borders.
- Touch targets: minimum 44×44 CSS px on mobile.
- Focus outlines: never `outline: none` without a visible replacement.

---

## 7. Layout Reference (App.vue shell)

```
.app-shell (flex-row)
├── .desktop-nav (72px sidebar, flex-column)  ← desktop only
│   ├── logo
│   ├── nav buttons
│   ├── spacer
│   └── dev badge
├── .app-main (flex: 1)
│   └── ChatView / BrainView / MemoryView / …
├── .mobile-bottom-nav (56px fixed-bottom)     ← mobile ≤640px
└── floating overlays: QuestBubble, ComboToast, QuestRewardCeremony
```

The mode-toggle pill lives **inside** the chat view's top toolbar container,
not as a fixed-position element outside the layout.

---

## 8. Enforcement

- **Lint rule** (future): `stylelint-declaration-strict-value` for color,
  z-index, border-radius.
- **Code review**: Reviewers must reject `position: fixed` for layout elements,
  bare z-index integers, and hardcoded hex colors.
- **Migration**: Existing violations are tracked in the backlog. New code must
  not introduce new violations.
