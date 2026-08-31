# AIP UI Reset — Phase A Design Specification

Status: design direction approved for implementation through the Dual Codex Executor.

## Scope and invariants

This reset replaces the presentation layer while keeping the existing React/Tauri
architecture, conversation lifecycle, model policy and Auto routing, runtime
readiness, persistence, accessibility behavior, overlays, pixel editor behavior,
and local-first security boundaries intact. UI copy remains Portuguese; this
document and source identifiers remain English. No remote publication is authorized.

The feature branch is based on the live `origin/main` tip. The objective recorded
`b0820fa7f55c39e6bfd5be42b45ebd2a51d41650`; live `origin/main` additionally contains
the docs-only local multimodal roadmap commit `bbaa93b0a3b52f42e6c1300a0a6c92bcfbf7c891`.

## Audit findings

- `apps/desktop/src/App.css` contains a newer token layer followed by a second,
  older dark UI layer. Later selectors overwrite earlier selectors, leaving two
  competing visual systems and raw per-component colors.
- `apps/desktop/src/theme.tsx` defaults to Inter and exposes system/Inter/Atkinson;
  the reset must add Times New Roman as the default while preserving safe font
  customization and persisted theme preferences.
- The current root renders product mark, open agent/navigation details,
  `ConversationList`, and runtime footer. The navigation details are before the
  chat region and consume a large fixed-height share. The pre-PR21 structure put
  agent tabs and `ConversationList` before secondary navigation, which is the
  correct priority reference.
- Conversation rows use small 30px controls and an always-present action column,
  but the current action treatment is visually indistinguishable from a disabled
  gray block. Menus already use a fixed/portal path in the component logic and
  must remain outside clipped list overflow.

## Reference extraction

Prompt Arena (read-only primary reference) uses a restrained editorial desktop
system: a 254px sidebar, 30/18px outer padding, compact 8px navigation gaps,
30–45px controls, visible 3px focus rings, one-pixel separators, and a flexible
conversation/history region. Its neutral surface ladder is canvas → surface →
raised → soft/muted, with warm off-white text and one quiet sand accent. Light mode
uses paper-toned neutrals rather than an inverted dark palette. Sections use 16/24/32px
rhythm, 10/18/28px radius tiers, and fixed floating menus that flip at viewport
edges. Responsive collapse happens near 900px and 680px.

Claude is a secondary interaction reference: content-first composition, calm
whitespace, a narrow readable measure, few containers, deliberate selected states,
subtle dividers, and menus that feel like part of the product rather than browser
chrome. No Claude branding, logo, trademark, or proprietary asset is copied.

The UI Pro Max search selected Minimalism/Swiss guidance for a professional desktop
AI workspace. Its applicable constraints are 4.5:1 text contrast, keyboard-visible
focus, 24px minimum pointer target (with larger comfortable controls here), semantic
labels, reduced motion, systematic breakpoints, and transform/opacity-only motion.

## AIP visual system

### Tokens

Use a four-point base scale with the following deliberate steps:

`--space-1: 4px`, `--space-2: 8px`, `--space-3: 12px`, `--space-4: 16px`,
`--space-5: 20px`, `--space-6: 24px`, `--space-8: 32px`, `--space-10: 40px`,
`--space-12: 48px`.

Type roles are 12/13px metadata, 14px labels, 16px body, 18px section titles,
22px page titles, 28px display titles. Body line-height is 1.5; long copy is capped
at 68ch. The base font is `"Times New Roman", Times, serif`; custom font choices
continue to flow through the existing allowlisted theme preference.

The new dark foundation is graphite and warm paper:

- canvas `#121314`; surface `#1a1b1d`; raised `#222427`; soft `#292b2f`;
  muted `#202226`;
- border `#383b40`; strong border `#555a61`;
- text `#f3f1ec`; muted text `#c1bdb5`; subtle text `#918d86`;
- accent `#d0aa72`; accent-strong `#efd09b`; accent ink `#241d14`;
- success `#9ac6a4`; warning `#ddbd76`; danger `#e0a09a`.

The light foundation is independently tuned:

- canvas `#f3f0ea`; surface `#fffdf8`; raised `#f8f4ed`; soft `#eee9df`;
  muted `#f5f0e7`;
- border `#d0c8bd`; strong border `#aa9f92`;
- text `#2c2925`; muted text `#5f5951`; subtle text `#766f67`;
- accent `#a8783e`; accent-strong `#855b2e`; accent ink `#fffaf1`;
- success `#2f7650`; warning `#8a5a00`; danger `#a44f48`.

Theme primary/secondary colors remain user-configurable via semantic variables;
component CSS must never hard-code screen-specific hex colors. Radius remains
configurable, but uses selective tiers: 4px controls, 8px rows/menus, 12px panels,
16px dialogs, and 24px only for intentionally prominent surfaces.

### Layout primitives

The application shell is a two-column grid with a 248px desktop sidebar and a
`minmax(0, 1fr)` workspace. The sidebar is a five-row grid:

`brand auto / agents auto / navigation auto / conversations minmax(0, 1fr) / footer auto`.

The conversation list owns the flexible row and its own scroll container. Navigation
is compact and can collapse; it must not become the flexible row. The main workspace
uses a readable max-width, a persistent page header, and one focused content column.

### Component families

- **Controls:** 40px standard and 48px comfortable heights, semantic buttons,
  custom select/menu/file picker visuals, visible focus, no native-select leakage.
- **Navigation:** compact section summaries, icon/label alignment, accent rail or
  inset state for active locations, readable inactive text.
- **Conversation rows:** 44px minimum hit target, title truncation, optional metadata,
  always-legible ellipsis button, distinct normal/hover/active/focus/menu-open
  states, and portal/floating menus with viewport collision handling.
- **Surfaces:** section separators and a small number of raised panels; no card wall.
- **Forms/settings:** aligned labels, helper text beside the relevant field, grouped
  sections, two-column grids that collapse at 900/680px, and purposeful empty states.
- **Overlay/pixel:** shared control tokens, transparent overlay-safe backgrounds,
  deliberate grouped toolbar intents (Drawing, Selection, Transform/View, History,
  File), and bounded selected-pet indicators.

### Motion

Use `--motion-in: 180ms`, `--motion-out: 120ms`, and `--motion-state: 160ms` with
ease-out/linear only where the cause requires it. Hover/focus and menu transitions
use opacity/transform; rows never reflow on interaction. The generation indicator
uses a restrained moving highlight only while generation is active. A single
`prefers-reduced-motion` rule and the existing persisted reduced-motion preference
must set animation/transition durations to near-zero and show the final legible text.

## Migration map

1. Replace `apps/desktop/src/App.css` with one layered foundation: reset/tokens,
   typography, layout, controls, navigation/chat, forms/settings, overlays, and
   motion. Remove obsolete duplicate selectors instead of adding an override pile.
2. Update `theme.tsx` default font and semantic light/dark token values while
   preserving storage keys, color validation, radius choices, and reduced-motion
   behavior. Add tests for Times New Roman default and token persistence.
3. Keep `App.tsx` behavior and component logic. Only make presentation-linked
   markup changes where needed for a semantic state hook or a generation-state
   class; retain all IPC, persistence, model, runtime, and security code.
4. Migrate sidebar/conversation selectors first, then shared controls and chat
   surfaces, then Memories/State/Profile/Appearance, Settings/Resources, and finally
   overlays/pixel editor. Keep portal menus out of clipped scroll containers.
5. Add focused contract tests for flexible sidebar structure, action accessibility,
   generation-only shiny state, reduced motion, and preservation of settings/model/
   overlay behavior without brittle pixel snapshots.

## Acceptance gates

The reset is ready for human review only when the chat list visibly dominates the
sidebar, each conversation row has clear states and an intentional always-visible
ellipsis affordance, dark/light surfaces have readable hierarchy, Times New Roman
renders without clipping, settings/forms/resources/pixel/overlay surfaces look
finished, and no functional regression is observed in static/automated checks.

Implementation remains local-only. Do not push, open/update a PR, merge, tag,
release, deploy, or perform destructive remote actions.
