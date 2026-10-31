# shadcn/ui — Production Design System and AI Implementation Contract

> **Status:** Source-anchored engineering reference, 2026-10-29. **Inspected upstream revision:** [`96da9cc7f1aa10580a3741664e2a77c721206c00`](https://github.com/shadcn-ui/ui/commit/96da9cc7f1aa10580a3741664e2a77c721206c00) (latest commit returned by the repository's commit listing on the research date; commit dated 2026-10-27). **Baseline:** `base-nova` + neutral color tokens + Tailwind CSS v4 + React/TypeScript. **Scope:** `apps/v4`, authored Base UI/Radix/React Aria registries, style CSS, official docs, and selected implementation files. **Evidence levels:** `[S]` inspected source at pinned revision; `[D]` official documentation read 2026-10-29; `[R]` additional project-wide recommendation, not a shadcn requirement; `[U]` unverified/depends on installed version. **Never interpret `[R]` values as upstream defaults.**

**Repository access limitation:** Git network cloning was unavailable in the execution environment (DNS failure), so this audit used GitHub's authenticated file-content and commit interfaces for pinned upstream source plus the public official docs. The three UI registries, registry architecture, `style-nova.css`/other style samples, and selected component implementations were inspected. It was **not** possible to execute the upstream monorepo, test every component API, or audit every example file line-by-line. Consequently the matrix below is a *registry-wide documented inventory*, but entries without direct implementation inspection are clearly marked `[registry]`; their precise primitive props and state-specific styling must not be assumed. This is an explicit qualification, not an assertion of full-source verification.

## Table of contents

1. [Operating contract and evidence](#1-operating-contract-and-evidence)
2. [Architecture, styles, and philosophy](#2-architecture-styles-and-philosophy)
3. [Exact neutral color tokens and CSS scaffold](#3-exact-neutral-color-tokens-and-css-scaffold)
4. [Typography, spacing, radius, elevation, and motion](#4-typography-spacing-radius-elevation-and-motion)
5. [Layout recipes and responsive behavior](#5-layout-recipes-and-responsive-behavior)
6. [Component inventory and individual specifications](#6-component-inventory-and-individual-specifications)
7. [Composition recipes and state machines](#7-composition-recipes-and-state-machines)
8. [Interaction, keyboard, and motion standards](#8-interaction-keyboard-and-motion-standards)
9. [WCAG 2.2 AA and test plan](#9-wcag-22-aa-and-test-plan)
10. [Themes, icons, imagery, and visual quality](#10-themes-icons-imagery-and-visual-quality)
11. [Frontend architecture and AI agent contract](#11-frontend-architecture-and-ai-agent-contract)
12. [Ten reference implementations](#12-ten-reference-implementations)
13. [Compliance checklist and verification log](#13-compliance-checklist-and-verification-log)

## 1. Operating contract and evidence

**Normative order for implementation:** (1) existing project's installed components and `components.json`, (2) pinned/installed registry source, (3) official component and theming docs for matching base/style, (4) this project's `[R]` conventions, (5) new design choices only after documenting why. When importing an upstream component in a real app, use the installed project's actual `@/components/ui/*` path; repository source uses `@/registry/bases/base/ui/*`, which is **not** an import path to copy into normal applications. Do not mix `base`, `radix`, `aria` versions of an interactive component in a page without deliberate migration.

**Primary sources:**

- [Registry architecture](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/README.md) `[S]`; [base registry index](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts) `[S]`; [radix index](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/radix/ui/_registry.ts) `[S]`; [aria index](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/aria/ui/_registry.ts) `[S]`.
- [Nova style](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css) `[S]`; [Vega style](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-vega.css) `[S]`; [Sera style](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-sera.css) `[S]`.
- [Theming and default neutral scaffold](https://ui.shadcn.com/docs/theming) `[D]`; [CLI](https://ui.shadcn.com/docs/cli) `[D]`; [Components catalog](https://ui.shadcn.com/docs/components) `[D]`; [Next.js installation](https://ui.shadcn.com/docs/installation/next) `[D]`; [Dark mode](https://ui.shadcn.com/docs/dark-mode/next) `[D]`; [React Aria announcement](https://ui.shadcn.com/docs/changelog/2026-07-react-aria) `[D]`.
- For component-specific verified source, use `https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/COMPONENT.tsx`; the generated table gives direct paths. For exact docs, use `https://ui.shadcn.com/docs/components/base/COMPONENT` (switch `base` to `radix` or `aria` when selecting the corresponding implementation).

**Important inventory arithmetic:** Base UI authored `ui/_registry.ts`: **65 `registry:ui` entries**; Radix: **64**; React Aria: **62**. Base includes legacy `toast` and a `form` registry entry without an implementation file; the registry count is not the count of installable `.tsx` components. `sonner` is the intended notification pattern for new work. The docs also include **recipes**, e.g. Data Table, Date Picker, Typography, and AI Elements, which do not map one-to-one to installable primitive names. Those are covered separately. Do not call any undocumented `variant` or `size` an official API.

## 2. Architecture, styles, and philosophy

### What shadcn/ui actually is `[S,D]`

It is a **source-distribution component registry**, not a locked component runtime. The CLI copies component source into the application; you own and can modify that source. `apps/v4/registry/bases/base`, `/radix`, and `/aria` author corresponding primitive-backed versions. `apps/v4/registry/styles/style-*.css` supplies style-layer rules; build output combines base and style (e.g. `base-nova`, `radix-nova`). `new-york-v4` is the separately maintained **legacy** registry, not a synonym for `base-nova`. The registry's `README.md` distinguishes hand-maintained sources from generated indexes, `styles/` and `public/r/`; never hand-edit generated registry files in the upstream repo.

**Primitive differences `[S,D]`:** `base` uses `@base-ui/react` (e.g. `ButtonPrimitive.Props`, Base UI dialog and select); `radix` uses Radix primitives; `aria` uses React Aria Components. Some wrappers are plain React elements; `Command` uses `cmdk`, charts use Recharts, calendar uses React DayPicker, carousel uses Embla, and resizable uses React Resizable Panels. React Aria is first-class as of the July 2026 changelog. **A Base UI `render` customization is not Radix `asChild`**. For example, legacy `new-york-v4/button.tsx` exports `asChild`, while inspected `bases/base/ui/button.tsx` uses `ButtonPrimitive.Props` and does **not** implement a separate `asChild` prop. Agent rule: inspect installed code before using composition APIs.

### Design philosophy `[R]` (derived guidelines, not upstream obligations)

- **Quiet, functional hierarchy:** one primary action per decision cluster; headings, labels and metadata create hierarchy before color does. Reserve saturated/semantic color for meaningful actions/status.
- **Separation by surfaces and space:** start with `bg-background`; use `bg-card` for panels, `bg-popover` for overlays, `bg-muted` for supporting areas. Borders establish structure; shadows indicate floating elevation, not every rectangle.
- **Composability over cloning:** use installed `Button`, `Field`, `Card`, `Table`, `Dialog`, `Sidebar`, `Empty`, etc.; compose domain wrappers (e.g. `MetricCard`) rather than reimplementing base controls.
- **Density is contextual:** dashboards and data tables can be compact; onboarding, high-stakes forms and marketing pages need more breathing room. Do not apply a global `scale(0.9)` or random `text-xs` to solve density.
- **Clarity under stress:** loading, empty, partial error, disabled, stale, pending, and saved states must have meaningful explanatory text; never represent status by color alone.
- **Aesthetic restraint:** use a single accent family per product, consistent icon language, 1px dividers, rounded corners from the active style and a small number of page-width modes; avoid gratuitous gradients, glass overlays and rainbow cards.

### Styles are **not interchangeable pixel values** `[S]`

| Inspected style | Source-backed distinctions | Implementation rule |
|---|---|---|
| `nova` (baseline) | Button `default` height `h-8`, `sm h-7`, `lg h-9`, `xs h-6`; `rounded-lg`; Card `rounded-xl`, `ring-1`, default spacing `--spacing(4)`; menu transitions commonly `duration-100`; primary/secondary/ghost/destructive/outline/link button variants. | Use for this document's exact component measurements. |
| `sera` | Square/`rounded-none` controls, uppercase tracked small buttons, e.g. default `h-10`, Card spacing `--spacing(8)`, distinct editorial hierarchy. | Do **not** copy Nova heights/radii into Sera. |
| `vega` | Separate authored CSS style, not assumed equivalent to Nova. | Inspect selected styles before measuring. |
| Other named styles | Docs and changelog list Vega, Nova, Maia, Lyra, Mira, Luma, Rhea, Sera. | Treat their concrete class metrics as **unverified here**; choose one and pin it. |

Visual feel is a project decision, not a universal shadcn “official aesthetic.” For this contract choose **Nova neutral**, a restrained modern product look. Avoid changing `components.json.style` mid-build.

## 3. Exact neutral color tokens and CSS scaffold

### 3.1 Official default neutral values `[D]`

The table below transcribes values from the **default neutral scaffold**, [theming docs](https://ui.shadcn.com/docs/theming). It is **not a claim** that every custom theme or style shares these values. `oklch()` values are CSS Color 4; keep their tokens intact instead of substituting arbitrary hex approximations.

| CSS variable | Light `:root` | Dark `.dark` | Intent / don't use for |
|---|---|---|---|
| `--background` | `oklch(1 0 0)` | `oklch(0.145 0 0)` | Page canvas; not a CTA |
| `--foreground` | `oklch(0.145 0 0)` | `oklch(0.985 0 0)` | Main text; not muted metadata |
| `--card` | `oklch(1 0 0)` | `oklch(0.205 0 0)` | Panel surface; not a transient overlay |
| `--card-foreground` | `oklch(0.145 0 0)` | `oklch(0.985 0 0)` | Card text |
| `--popover` | `oklch(1 0 0)` | `oklch(0.205 0 0)` | Floating layer |
| `--popover-foreground` | `oklch(0.145 0 0)` | `oklch(0.985 0 0)` | Floating content text |
| `--primary` | `oklch(0.205 0 0)` | `oklch(0.922 0 0)` | Main interaction; not every selected row |
| `--primary-foreground` | `oklch(0.985 0 0)` | `oklch(0.205 0 0)` | On primary |
| `--secondary` | `oklch(0.97 0 0)` | `oklch(0.269 0 0)` | Supporting filled control |
| `--secondary-foreground` | `oklch(0.205 0 0)` | `oklch(0.985 0 0)` | On secondary |
| `--muted` | `oklch(0.97 0 0)` | `oklch(0.269 0 0)` | Supporting background |
| `--muted-foreground` | `oklch(0.556 0 0)` | `oklch(0.708 0 0)` | Secondary text; check contrast on small text |
| `--accent` | `oklch(0.97 0 0)` | `oklch(0.269 0 0)` | Hover/selection surface |
| `--accent-foreground` | `oklch(0.205 0 0)` | `oklch(0.985 0 0)` | On accent |
| `--destructive` | `oklch(0.577 0.245 27.325)` | `oklch(0.704 0.191 22.216)` | Dangerous actions/errors, not warning/success |
| `--border` | `oklch(0.922 0 0)` | `oklch(1 0 0 / 10%)` | Structural separators |
| `--input` | `oklch(0.922 0 0)` | `oklch(1 0 0 / 15%)` | Form borders |
| `--ring` | `oklch(0.708 0 0)` | `oklch(0.556 0 0)` | Focus indication (contrast must be tested) |
| `--chart-1` | `oklch(0.646 0.222 41.116)` | `oklch(0.488 0.243 264.376)` | Series 1 |
| `--chart-2` | `oklch(0.6 0.118 184.704)` | `oklch(0.696 0.17 162.48)` | Series 2 |
| `--chart-3` | `oklch(0.398 0.07 227.392)` | `oklch(0.769 0.188 70.08)` | Series 3 |
| `--chart-4` | `oklch(0.828 0.189 84.429)` | `oklch(0.627 0.265 303.9)` | Series 4 |
| `--chart-5` | `oklch(0.769 0.188 70.08)` | `oklch(0.645 0.246 16.439)` | Series 5 |
| `--sidebar` | `oklch(0.985 0 0)` | `oklch(0.205 0 0)` | Sidebar surface |
| `--sidebar-foreground` | `oklch(0.145 0 0)` | `oklch(0.985 0 0)` | Sidebar text |
| `--sidebar-primary` | `oklch(0.205 0 0)` | `oklch(0.488 0.243 264.376)` | High-priority sidebar elements |
| `--sidebar-primary-foreground` | `oklch(0.985 0 0)` | `oklch(0.985 0 0)` | On sidebar-primary |
| `--sidebar-accent` | `oklch(0.97 0 0)` | `oklch(0.269 0 0)` | Sidebar hover/current item |
| `--sidebar-accent-foreground` | `oklch(0.205 0 0)` | `oklch(0.985 0 0)` | On sidebar-accent |
| `--sidebar-border` | `oklch(0.922 0 0)` | `oklch(1 0 0 / 10%)` | Sidebar separators |
| `--sidebar-ring` | `oklch(0.708 0 0)` | `oklch(0.556 0 0)` | Sidebar focus |

**Semantic translation:** `bg-background text-foreground`; `bg-card text-card-foreground`; `bg-popover text-popover-foreground`; `bg-primary text-primary-foreground`; `bg-muted text-muted-foreground`. **Never** put `text-primary-foreground` on the page background by accident. `chart-*` is a *categorical data* palette, not a general dashboard decoration palette.

### 3.2 Complete app theme scaffold `[D]`

The official docs combine Tailwind v4 with `shadcn/tailwind.css`. This compact implementation contains the **full standard neutral palette** as above, with all Tailwind mappings shown so it can be used independently.

```css
/* app/globals.css — neutral baseline from official theming docs */
@import "tailwindcss";
@import "shadcn/tailwind.css";
@import "tw-animate-css";
@custom-variant dark (&:is(.dark *));
@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);
  --color-sidebar: var(--sidebar);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-ring: var(--sidebar-ring);
  --radius-sm: calc(var(--radius) * .6);
  --radius-md: calc(var(--radius) * .8);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) * 1.4);
  --radius-2xl: calc(var(--radius) * 1.8);
  --radius-3xl: calc(var(--radius) * 2.2);
  --radius-4xl: calc(var(--radius) * 2.6);
}
:root {
  --radius: 0.625rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --secondary: oklch(0.97 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.97 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --accent: oklch(0.97 0 0);
  --accent-foreground: oklch(0.205 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --chart-1: oklch(0.646 0.222 41.116);
  --chart-2: oklch(0.6 0.118 184.704);
  --chart-3: oklch(0.398 0.07 227.392);
  --chart-4: oklch(0.828 0.189 84.429);
  --chart-5: oklch(0.769 0.188 70.08);
  --sidebar: oklch(0.985 0 0);
  --sidebar-foreground: oklch(0.145 0 0);
  --sidebar-primary: oklch(0.205 0 0);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.97 0 0);
  --sidebar-accent-foreground: oklch(0.205 0 0);
  --sidebar-border: oklch(0.922 0 0);
  --sidebar-ring: oklch(0.708 0 0);
}
.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.205 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --accent: oklch(0.269 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.704 0.191 22.216);
  --border: oklch(1 0 0 / 10%);
  --input: oklch(1 0 0 / 15%);
  --ring: oklch(0.556 0 0);
  --chart-1: oklch(0.488 0.243 264.376);
  --chart-2: oklch(0.696 0.17 162.48);
  --chart-3: oklch(0.769 0.188 70.08);
  --chart-4: oklch(0.627 0.265 303.9);
  --chart-5: oklch(0.645 0.246 16.439);
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
  --sidebar-primary: oklch(0.488 0.243 264.376);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.269 0 0);
  --sidebar-accent-foreground: oklch(0.985 0 0);
  --sidebar-border: oklch(1 0 0 / 10%);
  --sidebar-ring: oklch(0.556 0 0);
}
@layer base {
  * { @apply border-border outline-ring/50; }
  body { @apply bg-background text-foreground; }
}
```

**Caveat:** This provides the docs' token scaffold. The extra imported `tw-animate-css` is declared in the inspected registry's base-style devDependencies / CSS and is needed if installed animated components reference its utilities. CLI is responsible for scaffolding dependencies and style-specific CSS; **run the CLI**, do not assume pasting this file alone installs Nova's `.cn-*` classes. The `--radius` token above is exactly **10px** (`0.625rem`) only if root font is 16px. Default document tokens have no global `--success` / `--warning`; add them intentionally after contrast testing.

### 3.3 Token ownership and usage `[R]`

| Tier | Examples | Ownership / rule |
|---|---|---|
| Semantic theme | `--background`, `--primary`, `--ring` | Change at the `:root` / `.dark` scope and test both modes. |
| Derived theme | `--radius-sm` through `--radius-4xl` | Do not hardcode multiple competing radius scales. |
| Tailwind foundation | `p-4`, `text-sm`, `md:`, `shadow-sm` | Core Tailwind CSS v4 defaults; **not shadcn-specific tokens**. |
| Style-specific classes | `.cn-button-size-default`, `.cn-card`, `.cn-dialog-content` | Source of exact Nova pixel behavior, not globally true across styles. |
| Domain semantic `[R]` | `--status-success`, `--status-warning`, `--focus-strong` | Define explicitly in application tokens before use. |

## 4. Typography, spacing, radius, elevation, and motion

### 4.1 Typography ladder `[R]` (Tailwind utilities, *not* a claimed official type scale)

| Role | Suggested Tailwind | Typical use | Constraint |
|---|---|---|---|
| Hero | `text-4xl sm:text-5xl font-semibold tracking-tight leading-tight` | Landing only | Never use in dense dashboards |
| Page title / `h1` | `text-2xl sm:text-3xl font-semibold tracking-tight` | Top of page | Exactly one primary H1 |
| Section / `h2` | `text-lg sm:text-xl font-semibold tracking-tight` | Divided panels | Level follows semantics |
| Card title | `text-base font-medium` | Nova Card's default `[S]` | Don't manually upsize all cards |
| Body | `text-sm leading-6` | Product UI text | Use `text-base` for long-form reading |
| Label | `text-sm font-medium` | Forms, controls | Programmatic `<label>` required |
| Auxiliary | `text-xs text-muted-foreground` | Timestamp/overline | Avoid for critical actions/errors |
| Code | `font-mono text-xs sm:text-sm` | JSON, keys, IDs | Respect `break-all`/overflow scroll |

**Exact Nova evidence:** `.cn-card-title` = `text-base leading-snug font-medium`; small Card title `text-sm`. `.cn-card-description` = `text-muted-foreground text-sm`; `.cn-button` = `text-sm font-medium`; `.cn-badge` = `text-xs font-medium`; `.cn-command-item` = `text-sm` ([Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css)). Do not project those onto Sera, whose buttons are uppercase `text-xs`.

**Rules `[R]`:** Use native `h1`–`h6` in order, not a div with bold text; line length 55–75 characters for longer prose (`max-w-prose`); `text-balance` for short headings and `text-pretty` for paragraphs; `min-w-0 truncate` for one-line names; allow wrapping for form validation and critical statuses. Never truncate the only visible value of a destructive action or an error.

### 4.2 Spacing and responsive scale `[R]`

Tailwind's default unit is **0.25rem (4px)**. Examples: `gap-1` 4px, `gap-2` 8px, `gap-3` 12px, `gap-4` 16px, `gap-6` 24px, `gap-8` 32px, `gap-12` 48px. These are **Tailwind default values**, not independently specified by shadcn. Use increments of 4px where possible. For dense dashboard rows use `gap-2` / `gap-3`; for form rows `gap-4`; between unrelated page sections `gap-6` / `gap-8`; page edge `px-4 sm:px-6 lg:px-8`. Use `space-y-*` within known stacks; do not stack `mt-6` and a parent's `gap-6` on the same separation.

| Pattern `[R]` | Mobile | Tablet | Desktop |
|---|---|---|---|
| App content gutter | `px-4` (16px) | `sm:px-6` (24px) | `lg:px-8` (32px) |
| Dashboard grid | 1 column | 2 at `sm` | 3–4 at `xl` |
| Form maximum width | `w-full` | `max-w-xl` (36rem) | 36rem or 42rem |
| Settings page | 1 column | 1 column | Nav `w-48` + content `minmax(0,1fr)` |
| Sidebar | Sheet below `md` `[S]` | Desktop at `md` | fixed/collapsible |
| Chart | width 100%, fixed responsive height | > | `min-w-0` in grid |
| Data table | horizontal scroll | dense + scroll if needed | whole width |

**Default Tailwind breakpoints `[R, Tailwind]`:** `sm` 40rem (640px), `md` 48rem (768px), `lg` 64rem (1024px), `xl` 80rem (1280px), `2xl` 96rem (1536px). These depend on theme configuration and are not guaranteed if overridden. Prefer container queries `@container` for independent panels and logical properties `ps-*`, `pe-*`, `ms-*`, `me-*` when supporting RTL.

### 4.3 Control measurements `[S]` — *Nova specifically*

| Element | Verified Nova classes / resulting dimensions (default 16px rem) | Design implication |
|---|---|---|
| Button `xs` | `h-6` 24px, `px-2`, `text-xs` | Dense toolbars only, not touch-primary |
| Button `sm` | `h-7` 28px, `px-2.5`, `text-[0.8rem]` | Dense actions |
| Button default | `h-8` 32px, `px-2.5` | Desktop action baseline |
| Button `lg` | `h-9` 36px, `px-2.5` | Primary form action |
| Icon button | `icon-xs size-6`, `icon-sm size-7`, `icon size-8`, `icon-lg size-9` | Use aria-label; enlarge target on touch |
| Default SVG in button | `size-4` (16px) unless explicitly sized | Don't add conflicting SVG sizes |
| Card | `rounded-xl`, `ring-1`, `py-(--card-spacing)`, default `--card-spacing: --spacing(4)` | 16px internal spacing |
| Small Card | `data-size=sm`, `--card-spacing: --spacing(3)` | 12px internal spacing |
| Badge | `h-5`, `px-2`, `py-0.5`, `text-xs` | State labels, not buttons |
| Avatar | `size-8` default; `sm size-6`; `lg size-10` | Lists/header/identity |
| Checkbox | `size-4` (16px) | Wrap in comfortable label target |
| Calendar | `[--cell-size:--spacing(7)]`, `p-2` | Cell approx 28px; add touch adaptation |
| Dialog overlay | `duration-100` on Nova overlay | Respect reduced motion |
| Menu / combobox list | `max-h-72`, popover content `min-w-36` for relevant primitives | Never clip outside viewport |

**Source:** [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css), [Base button](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button.tsx), [Base card](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/card.tsx).

### 4.4 Borders, shadows, stacking, icons, animation `[R unless tagged]`

- Radius: derive from `--radius: 0.625rem` using the official multipliers: sm `.6` (6px), md `.8` (8px), lg `1` (10px), xl `1.4` (14px), 2xl `1.8` (18px), 3xl `2.2` (22px), 4xl `2.6` (26px) at 16px root `[D]`. Nova's `rounded-lg` / `rounded-xl` classes select these variables, not fixed browser-radius constants.
- Border `border` means 1 CSS pixel at Tailwind defaults. Prefer `border-border` on section boundaries; cards in inspected Nova use subtle `ring-foreground/10 ring-1` rather than a heavy border/shadow `[S]`. Use `shadow-md` for floating menus when the actual style does; avoid shadows on every panel.
- **Stacking conventions `[R]`**: `z-0` background, `z-10` sticky child/control, `z-20` sticky nav, `z-30` nested floating helper, `z-40` overlay backdrops, `z-50` popovers/dialogs. Actual portalled primitives manage stacking; these are *policy targets*, not asserted Nova source values. For nested menus/drawers, ensure one consistent overlay ownership and test stacking.
- **Icons `[R]`**: Lucide when project initialized with `iconLibrary: lucide` (repo example `apps/v4/components.json`); typical inline icon `size-4` (16px) matching Nova button defaults `[S]`, muted utility icons `size-3.5` (14px), large feature illustration `size-6` (24px). Respect inherited `strokeWidth`; do not combine four icon packs because examples reference several icon placeholders in the design source.
- **Motion `[R]`**: quick hover 100–150ms; panels 150–200ms; nonessential feedback 150–250ms with standard `ease-out`. **Nova confirmed** overlay/menu classes include `duration-100` in style CSS `[S]`. Do not invent an upstream animation token or blanket `transition-all duration-300`. Prefer `transition-colors`; use `motion-reduce:transition-none motion-reduce:animate-none` where possible.
- **Opacity `[R]`**: `disabled:opacity-50` is source-supported for Button `[S]`; never reduce the opacity of already-muted essential instructions below readable contrast. Native `disabled`/`aria-disabled` semantics must be correct.

## 5. Layout recipes and responsive behavior

### 5.1 Application shell `[R]` with source-verified Sidebar measurements `[S]`

Inspected `bases/base/ui/sidebar.tsx`: `SIDEBAR_WIDTH="16rem"` (256px), `SIDEBAR_WIDTH_MOBILE="18rem"` (288px), `SIDEBAR_WIDTH_ICON="3rem"` (48px), `SIDEBAR_COOKIE_NAME="sidebar_state"`, cookie max-age **7 days**, keyboard shortcut **Cmd/Ctrl+B**, desktop starts at `md`; mobile uses `Sheet`. `Sidebar` supports `side: left|right`, `variant: sidebar|floating|inset`, `collapsible: offcanvas|icon|none`, and provider accepts `defaultOpen`, `open`, `onOpenChange` `[S]`. Don't hardcode 320px sidebar; don't wrap `SidebarProvider` around only half the app. [Sidebar source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sidebar.tsx).

```text
Desktop >= md:                   Mobile < md:
┌──────────────┬────────────────┐ ┌───────────────────────────┐
│ Sidebar      │ Header         │ │ Header   [menu] [account] │
│ 16rem        ├────────────────┤ ├───────────────────────────┤
│              │ Page body      │ │ Page body                 │
│              │ max-w-7xl      │ │ px-4                      │
└──────────────┴────────────────┘ └───────────────────────────┘
                                  Sidebar opens as 18rem Sheet
```

```tsx
// Layout idea; concrete Sidebar usage in reference implementation §12.2.
<main className="min-w-0 flex-1 overflow-x-hidden">
  <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-4 py-6 sm:px-6 lg:px-8">
    <header className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0"><h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
        <p className="text-sm text-muted-foreground">Track activity and health.</p></div>
      <div className="flex flex-wrap items-center gap-2">{/* actions */}</div>
    </header>
    {/* page sections */}
  </div>
</main>
```

### 5.2 Specific compositions `[R]`

| Screen | Preferred structural skeleton | States to handle |
|---|---|---|
| SaaS overview | `Sidebar` → header → `grid gap-4 sm:grid-cols-2 xl:grid-cols-4` metric cards → chart + activity → table | stale metrics, chart empty, partial fetch error |
| Developer console | narrow nav + content `max-w-5xl` + code blocks; split resizable only when both panels independently useful | long code, copy success, network errors |
| Settings | `max-w-4xl`, internal nav + `FieldSet/FieldGroup`, sections `space-y-6`, actions sticky **only if necessary** | validation, dirty, saving, saved, denied |
| Auth | `min-h-svh grid place-items-center px-4`, card `w-full max-w-sm`, single column fields | credential errors, pending, reset, MFA |
| CRUD | heading + filters + action; `overflow-x-auto` table; rows with action menu; `Dialog` create/edit; `AlertDialog` delete | zero data, no filter matches, busy, rollback |
| Search/command | a labeled input + results list with selected row + keyboard help; command palette in dialog | no results, partial query, keyboard focus |
| Files | breadcrumb + action toolbar + list/grid toggle, persistent selection, item context menu | empty folder, upload errors, access-denied |
| Chat | bounded conversation column, auto-scrolling only when user at bottom, composer pinned at bottom | streaming, stop, retry, file upload, code overflow |
| Onboarding | step heading and progress + one primary question per step | backtracking, partial saved, skip, error |

### 5.3 Overflow and density `[R]`

Always put `min-w-0` on flex/grid descendants containing long text. Use `overflow-x-auto` around `Table` instead of shrinking every cell to 60px. Use `max-h-[min(70vh,40rem)] overflow-y-auto` for long modal body; maintain accessible title and actions. Apply `sticky top-0 z-20 bg-background/95 backdrop-blur` only where content remains reachable and background contrast remains sound. For touch-specific controls, wrap the small visible icon with an **at least 24×24 CSS-pixel pointer target** as WCAG 2.2 AA baseline (exceptions exist), and aim for **44×44** where practical `[R]`. Avoid fixed body widths; test at 320px, 360px, 768px, 1024px, 1440px, 200% zoom and landscape mobile.

### 5.4 Responsive page recipe `[R]`

```tsx
<div className="mx-auto w-full max-w-7xl px-4 sm:px-6 lg:px-8">
  <div className="flex flex-col gap-4 py-6 sm:flex-row sm:items-center sm:justify-between">
    <h1 className="text-2xl font-semibold tracking-tight">Projects</h1>
    <div className="flex flex-wrap gap-2">{/* toolbar */}</div>
  </div>
  <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">{/* cards */}</div>
  <div className="mt-6 min-w-0 overflow-x-auto">{/* table */}</div>
</div>
```

## 6. Component inventory and individual specifications

The following is the **complete 65-name Base UI `registry:ui` index** at the pinned SHA, in registry order. Each section includes source-verified exported anatomy (for files fetched during the audit), target use, visual treatment, keyboard/accessibility responsibilities, a representative TSX usage **fragment**, and a specific anti-pattern. Fragments assume imports from `@/components/ui/{name}` plus state variables/handlers in scope; they are **illustrative compositions**, not a compiled stand-alone app. Fragments involving advanced, version-dependent primitives are explicitly advisory and require local API verification. **States**: all controls must be inspected for default, hover, focus-visible, active/selected, disabled, invalid, loading and success *where semantically meaningful*; absence of a built-in loading/success variant means composition, not an invented prop. Standard fallback: disable the active action, show `Spinner` with meaningful text, and announce success/error.

**Source provenance key:** `source-file verified` means the actual file was fetched or source portions read at pinned commit; `registry-only` means only its registry entry was inspected. An export listed below is **not necessarily a supported prop**; consult the corresponding source/API for exact primitive prop types. Do not paste `Base UI render={...}` into Radix/Aria projects without checking the relevant base documentation.

| Component | What it solves | Source | Docs |
|---|---|---|---|
| `accordion` | Grouped disclosure, FAQ, settings help | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/accordion.tsx) | [guide](https://ui.shadcn.com/docs/components/base/accordion) |
| `alert` | Inline informational or destructive notice | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/alert.tsx) | [guide](https://ui.shadcn.com/docs/components/base/alert) |
| `alert-dialog` | Confirmation for irreversible, consequential actions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/alert-dialog.tsx) | [guide](https://ui.shadcn.com/docs/components/base/alert-dialog) |
| `aspect-ratio` | Preserve stable image/video framing | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/aspect-ratio.tsx) | [guide](https://ui.shadcn.com/docs/components/base/aspect-ratio) |
| `avatar` | Identity thumbnail with fallback | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/avatar.tsx) | [guide](https://ui.shadcn.com/docs/components/base/avatar) |
| `badge` | Compact metadata, category or state | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/badge.tsx) | [guide](https://ui.shadcn.com/docs/components/base/badge) |
| `breadcrumb` | Reflect location in hierarchy | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/breadcrumb.tsx) | [guide](https://ui.shadcn.com/docs/components/base/breadcrumb) |
| `button` | Primary and secondary actions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button.tsx) | [guide](https://ui.shadcn.com/docs/components/base/button) |
| `button-group` | Group related actions or segmented controls | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button-group.tsx) | [guide](https://ui.shadcn.com/docs/components/base/button-group) |
| `calendar` | Calendar and date selection surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/calendar.tsx) | [guide](https://ui.shadcn.com/docs/components/base/calendar) |
| `card` | Related content surface and actions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/card.tsx) | [guide](https://ui.shadcn.com/docs/components/base/card) |
| `carousel` | Horizontally navigable content sequence | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/carousel.tsx) | [guide](https://ui.shadcn.com/docs/components/base/carousel) |
| `chart` | Themed wrapper around Recharts | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/chart.tsx) | [guide](https://ui.shadcn.com/docs/components/base/chart) |
| `checkbox` | Binary or multi-selection | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/checkbox.tsx) | [guide](https://ui.shadcn.com/docs/components/base/checkbox) |
| `collapsible` | Small reversible show/hide region | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/collapsible.tsx) | [guide](https://ui.shadcn.com/docs/components/base/collapsible) |
| `combobox` | Searchable choice/typed selection | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/combobox.tsx) | [guide](https://ui.shadcn.com/docs/components/base/combobox) |
| `command` | Command launcher and searchable results | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/command.tsx) | [guide](https://ui.shadcn.com/docs/components/base/command) |
| `context-menu` | Contextual action list on surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/context-menu.tsx) | [guide](https://ui.shadcn.com/docs/components/base/context-menu) |
| `dialog` | Modal conversation or editing task | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/dialog.tsx) | [guide](https://ui.shadcn.com/docs/components/base/dialog) |
| `drawer` | Swipeable sheet-like secondary panel | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/drawer.tsx) | [guide](https://ui.shadcn.com/docs/components/base/drawer) |
| `dropdown-menu` | Common action/context menu | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/dropdown-menu.tsx) | [guide](https://ui.shadcn.com/docs/components/base/dropdown-menu) |
| `empty` | Purposeful no-data/failure surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/empty.tsx) | [guide](https://ui.shadcn.com/docs/components/base/empty) |
| `field` | Composable accessible form layout | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/field.tsx) | [guide](https://ui.shadcn.com/docs/components/base/field) |
| `form` | Documented forms integration pattern; not a base registry .tsx file | [registry entry](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts) | [guide](https://ui.shadcn.com/docs/components/base/form) |
| `hover-card` | Preview supplementary object on hover/focus | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/hover-card.tsx) | [guide](https://ui.shadcn.com/docs/components/base/hover-card) |
| `input` | Single-line text/value entry | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input.tsx) | [guide](https://ui.shadcn.com/docs/components/base/input) |
| `input-group` | Input with inline or block addons/actions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input-group.tsx) | [guide](https://ui.shadcn.com/docs/components/base/input-group) |
| `input-otp` | One-time code entry | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input-otp.tsx) | [guide](https://ui.shadcn.com/docs/components/base/input-otp) |
| `item` | Standard clickable/nonclickable list row | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/item.tsx) | [guide](https://ui.shadcn.com/docs/components/base/item) |
| `label` | Accessible control caption | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/label.tsx) | [guide](https://ui.shadcn.com/docs/components/base/label) |
| `menubar` | Desktop-style action navigation menus | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/menubar.tsx) | [guide](https://ui.shadcn.com/docs/components/base/menubar) |
| `navigation-menu` | Site/global navigation with disclosures | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/navigation-menu.tsx) | [guide](https://ui.shadcn.com/docs/components/base/navigation-menu) |
| `pagination` | Paged dataset navigation | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/pagination.tsx) | [guide](https://ui.shadcn.com/docs/components/base/pagination) |
| `popover` | Nonmodal anchored interactive surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/popover.tsx) | [guide](https://ui.shadcn.com/docs/components/base/popover) |
| `progress` | Quantified task completion | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/progress.tsx) | [guide](https://ui.shadcn.com/docs/components/base/progress) |
| `radio-group` | Choose one from a small discrete set | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/radio-group.tsx) | [guide](https://ui.shadcn.com/docs/components/base/radio-group) |
| `resizable` | Adjustable multi-panel split view | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/resizable.tsx) | [guide](https://ui.shadcn.com/docs/components/base/resizable) |
| `scroll-area` | Themed scrollable viewport | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/scroll-area.tsx) | [guide](https://ui.shadcn.com/docs/components/base/scroll-area) |
| `select` | Single-value dropdown selection | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/select.tsx) | [guide](https://ui.shadcn.com/docs/components/base/select) |
| `separator` | Visual or semantic separation | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/separator.tsx) | [guide](https://ui.shadcn.com/docs/components/base/separator) |
| `sheet` | Responsive slide-over navigation or editor | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sheet.tsx) | [guide](https://ui.shadcn.com/docs/components/base/sheet) |
| `sidebar` | Application navigation system | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sidebar.tsx) | [guide](https://ui.shadcn.com/docs/components/base/sidebar) |
| `skeleton` | Loading placeholder mirroring layout | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/skeleton.tsx) | [guide](https://ui.shadcn.com/docs/components/base/skeleton) |
| `slider` | Bounded numeric range input | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/slider.tsx) | [guide](https://ui.shadcn.com/docs/components/base/slider) |
| `sonner` | Transient toast announcements | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sonner.tsx) | [guide](https://ui.shadcn.com/docs/components/base/sonner) |
| `spinner` | In-progress activity ornament | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/spinner.tsx) | [guide](https://ui.shadcn.com/docs/components/base/spinner) |
| `switch` | Immediately change binary setting | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/switch.tsx) | [guide](https://ui.shadcn.com/docs/components/base/switch) |
| `table` | Semantic tabular information | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/table.tsx) | [guide](https://ui.shadcn.com/docs/components/base/table) |
| `tabs` | Switch between peer views | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/tabs.tsx) | [guide](https://ui.shadcn.com/docs/components/base/tabs) |
| `textarea` | Long-form multiline text | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/textarea.tsx) | [guide](https://ui.shadcn.com/docs/components/base/textarea) |
| `toast` | Deprecated legacy registry entry (no new Base UI implementation file) | [registry entry](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts) | [guide](https://ui.shadcn.com/docs/components/base/toast) |
| `toggle` | One independent pressed/unpressed action | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/toggle.tsx) | [guide](https://ui.shadcn.com/docs/components/base/toggle) |
| `toggle-group` | Mutually related pressed options | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/toggle-group.tsx) | [guide](https://ui.shadcn.com/docs/components/base/toggle-group) |
| `tooltip` | Short nonessential explanation for a control | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/tooltip.tsx) | [guide](https://ui.shadcn.com/docs/components/base/tooltip) |
| `kbd` | Keyboard shortcut label | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/kbd.tsx) | [guide](https://ui.shadcn.com/docs/components/base/kbd) |
| `native-select` | Native OS choice list | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/native-select.tsx) | [guide](https://ui.shadcn.com/docs/components/base/native-select) |
| `direction` | Direction provider / RTL support | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/direction.tsx) | [guide](https://ui.shadcn.com/docs/components/base/direction) |
| `attachment` | Upload/download file surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/attachment.tsx) | [guide](https://ui.shadcn.com/docs/components/base/attachment) |
| `bubble` | Chat bubble and reactions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/bubble.tsx) | [guide](https://ui.shadcn.com/docs/components/base/bubble) |
| `message-scroller` | Scrollable message collection | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/message-scroller.tsx) | [guide](https://ui.shadcn.com/docs/components/base/message-scroller) |
| `questionnaire` | Guided AI/form questions | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/questionnaire.tsx) | [guide](https://ui.shadcn.com/docs/components/base/questionnaire) |
| `marker` | Inline marker | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/marker.tsx) | [guide](https://ui.shadcn.com/docs/components/base/marker) |
| `trace` | Expandable AI reasoning/tool trace surface | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/trace.tsx) | [guide](https://ui.shadcn.com/docs/components/base/trace) |
| `code-block` | Readable/copyable code output | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/code-block.tsx) | [guide](https://ui.shadcn.com/docs/components/base/code-block) |
| `message` | Chat message layout | [source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/message.tsx) | [guide](https://ui.shadcn.com/docs/components/base/message) |

### Component contracts (one per registry entry)

**Individual JSX code fences below are minimal composition fragments rather than standalone modules: imports, data and event handlers must be supplied by the consuming application. The ten complete page-level examples in §12 show component composition and state handling. Do not ship inert fragment actions as live UI.**

#### 6.01. `accordion`

**Use:** Grouped disclosure, FAQ, settings help. **Anatomy / exports `[S]`:** Accordion, AccordionItem, AccordionTrigger, AccordionContent. **Visual / states:** `AccordionTrigger` with text; collapsible item body; border-b between Nova items. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Enter/Space toggles a focused trigger; ensure headings express hierarchy. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not put all primary content behind accordions.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/accordion.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/accordion) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Accordion><AccordionItem value="policy"><AccordionTrigger>Policy</AccordionTrigger><AccordionContent>Policy details.</AccordionContent></AccordionItem></Accordion>
```

#### 6.02. `alert`

**Use:** Inline informational or destructive notice. **Anatomy / exports `[S]`:** Alert, AlertTitle, AlertDescription, AlertAction. **Visual / states:** Default and destructive semantics in Nova CSS; bordered grid, icon may be present. **Verified style classes:** default, destructive.

**Keyboard/accessibility & responsive:** For newly arriving actionable errors use `role="alert"` or appropriate live region; not every notice needs interruption. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use an Alert as a substitute for field errors.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/alert.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/alert) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Alert><AlertTitle>Changes saved</AlertTitle><AlertDescription>Your settings are up to date.</AlertDescription></Alert>
```

#### 6.03. `alert-dialog`

**Use:** Confirmation for irreversible, consequential actions. **Anatomy / exports `[S]`:** AlertDialog, AlertDialogTrigger, AlertDialogPortal, AlertDialogOverlay, AlertDialogContent, AlertDialogHeader, AlertDialogMedia, AlertDialogTitle, AlertDialogDescription, AlertDialogFooter, AlertDialogCancel, AlertDialogAction. **Visual / states:** Modal overlay + title, description and clear safe/destructive decision. **Verified API:** `AlertDialogContent size="default" | "sm"`; `AlertDialogAction` is a `Button`, not an automatic close action.

**Keyboard/accessibility & responsive:** Trap focus, allow safe cancel, restore focus, avoid accidental destructive default. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use Dialog for highly destructive confirmation without explicit consequences.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/alert-dialog.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/alert-dialog) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<AlertDialog><AlertDialogTrigger render={<Button variant="destructive" />}>Delete</AlertDialogTrigger><AlertDialogContent><AlertDialogHeader><AlertDialogTitle>Delete this record?</AlertDialogTitle><AlertDialogDescription>This cannot be undone.</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction>Delete</AlertDialogAction></AlertDialogFooter></AlertDialogContent></AlertDialog>
```

#### 6.04. `aspect-ratio`

**Use:** Preserve stable image/video framing. **Anatomy / exports `[S]`:** AspectRatio. **Visual / states:** Wrapper receives a ratio; image fills frame with object-cover. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Provide alt text for informative imagery; support responsive width. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not distort images with fixed width and height.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/aspect-ratio.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/aspect-ratio) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<AspectRatio ratio={16 / 9}><img src="/preview.webp" alt="Application preview" className="size-full object-cover" /></AspectRatio>
```

#### 6.05. `avatar`

**Use:** Identity thumbnail with fallback. **Anatomy / exports `[S]`:** Avatar, AvatarImage, AvatarFallback, AvatarGroup, AvatarGroupCount, AvatarBadge. **Visual / states:** Nova default 32px, small 24px, large 40px; round media and fallbacks. **Exact:** size default/sm/lg.

**Keyboard/accessibility & responsive:** Pair decorative avatar with nearby visible name; fallback must not expose private information. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use initials as the only accessible account identity.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/avatar.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/avatar) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Avatar><AvatarImage src="/avatar.png" alt="" /><AvatarFallback>AL</AvatarFallback></Avatar>
```

#### 6.06. `badge`

**Use:** Compact metadata, category or state. **Anatomy / exports `[S]`:** Badge, badgeVariants. **Visual / states:** Nova variants default, secondary, outline, destructive, ghost, link [style CSS]; default 20px height. **Verified style classes:** default, secondary, outline, destructive, ghost, link.

**Keyboard/accessibility & responsive:** Text must describe status; avoid unreadable color-only meaning. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use badge for a primary button.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/badge.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/badge) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Badge variant="secondary">Draft</Badge>
```

#### 6.07. `breadcrumb`

**Use:** Reflect location in hierarchy. **Anatomy / exports `[S]`:** Breadcrumb, BreadcrumbList, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator, BreadcrumbEllipsis. **Visual / states:** Muted ancestors; current page foreground; short separator icon. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use `aria-label="Breadcrumb"` and aria-current for active page via wrapper; collapse only hidden ancestors. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not a replacement for history navigation.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/breadcrumb.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/breadcrumb) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Breadcrumb><BreadcrumbList><BreadcrumbItem><BreadcrumbLink href="/">Home</BreadcrumbLink></BreadcrumbItem><BreadcrumbSeparator /><BreadcrumbItem><BreadcrumbPage>Projects</BreadcrumbPage></BreadcrumbItem></BreadcrumbList></Breadcrumb>
```

#### 6.08. `button`

**Use:** Primary and secondary actions. **Anatomy / exports `[S]`:** Button, buttonVariants. **Visual / states:** Nova variants default, outline, secondary, ghost, destructive, link; sizes xs, sm, default, lg, icon-xs, icon-sm, icon, icon-lg. **Exact:** default/outline/secondary/ghost/destructive/link; xs/sm/default/lg/icon-xs/icon-sm/icon/icon-lg.

**Keyboard/accessibility & responsive:** Native button or Base UI button props; label icon-only control; disabled and loading correctly; preserve focus ring. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Never use `asChild` with inspected Base UI Button; that's a different API.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/button) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Button variant="default" size="default" type="submit">Save changes</Button>
```

#### 6.09. `button-group`

**Use:** Group related actions or segmented controls. **Anatomy / exports `[S]`:** ButtonGroup, ButtonGroupSeparator, ButtonGroupText, buttonGroupVariants. **Visual / states:** Horizontal/vertical layouts; styled adjacent controls, text and separator. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Make each action self-describing; keep proper group label if selection. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use visual grouping in place of `aria-pressed` selection semantics.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button-group.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/button-group) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<ButtonGroup><Button variant="outline">Day</Button><Button variant="outline">Week</Button></ButtonGroup>
```

#### 6.10. `calendar`

**Use:** Calendar and date selection surface. **Anatomy / exports `[S]`:** Calendar, CalendarDayButton. **Visual / states:** React DayPicker-backed; Nova cell size token approx 28px. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Ensure date is accessible as date; min/max and unavailable dates explicit. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** For a date input, prefer an appropriately labeled Date Picker recipe.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/calendar.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/calendar) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
import * as React from "react";
import { Calendar } from "@/components/ui/calendar";

export function DateSelection() {
  const [date, setDate] = React.useState<Date | undefined>();
  return <Calendar mode="single" selected={date} onSelect={setDate} />;
}
```

#### 6.11. `card`

**Use:** Related content surface and actions. **Anatomy / exports `[S]`:** Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, CardFooter. **Visual / states:** Nova rounded-xl ring-1, 16px standard card spacing; small size is 12px. **Exact:** Card `size="default" | "sm"`; Header/Title/Description/Action/Content/Footer.

**Keyboard/accessibility & responsive:** A Card is a div, not inherently interactive; link/button must be real element. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not make the whole card clickable by div onClick alone.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/card.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/card) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Card><CardHeader><CardTitle>Traffic</CardTitle><CardDescription>Past 30 days</CardDescription></CardHeader><CardContent>12,408 visits</CardContent></Card>
```

#### 6.12. `carousel`

**Use:** Horizontally navigable content sequence. **Anatomy / exports `[S]`:** Carousel, CarouselContent, CarouselItem, CarouselPrevious, CarouselNext, useCarousel. **Visual / states:** Embla-based, rounded previous/next controls. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Accessible previous/next names, keyboard support, no auto-advance for essential info. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not appropriate for primary navigation or data tables.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/carousel.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/carousel) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Carousel><CarouselContent><CarouselItem>First</CarouselItem><CarouselItem>Second</CarouselItem></CarouselContent><CarouselPrevious /><CarouselNext /></Carousel>
```

#### 6.13. `chart`

**Use:** Themed wrapper around Recharts. **Anatomy / exports `[S]`:** ChartContainer, ChartTooltip, ChartTooltipContent, ChartLegend, ChartLegendContent, ChartStyle. **Visual / states:** Wrapper uses `aspect-video` unless resized; `chart-1..5` thematic series; Recharts is a separate dependency. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Always give text summary/table, accessible label, units and truthful axes. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not present inaccessible unlabeled visualization as sole insight.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/chart.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/chart) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
import { LineChart, Line } from "recharts";
import { ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
const points = [{ date: "Mon", visits: 42 }, { date: "Tue", visits: 66 }];
export function VisitsTrend() {
  return (
    <ChartContainer config={{ visits: { label: "Visits", color: "var(--chart-1)" } }} className="h-64 w-full">
      <LineChart data={points} accessibilityLayer>
        <Line dataKey="visits" stroke="var(--color-visits)" />
        <ChartTooltip content={<ChartTooltipContent />} />
      </LineChart>
    </ChartContainer>
  );
}
```

#### 6.14. `checkbox`

**Use:** Binary or multi-selection. **Anatomy / exports `[S]`:** Checkbox. **Visual / states:** Nova visible square 16px, checked primary fill. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Associate label/id; Space toggles, reflect indeterminate separately when applicable. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use a checkbox for immediate on/off settings when Switch is clearer.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/checkbox.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/checkbox) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<div className="flex items-center gap-2"><Checkbox id="terms" /><Label htmlFor="terms">I agree</Label></div>
```

#### 6.15. `collapsible`

**Use:** Small reversible show/hide region. **Anatomy / exports `[S]`:** Collapsible, CollapsibleTrigger, CollapsibleContent. **Visual / states:** Trigger/content composition; content toggles visibility. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Enter/Space toggle, `aria-expanded` correct via primitive. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not conceal validation errors inside closed collapsibles.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/collapsible.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/collapsible) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Collapsible><CollapsibleTrigger>Advanced options</CollapsibleTrigger><CollapsibleContent>Additional controls</CollapsibleContent></Collapsible>
```

#### 6.16. `combobox`

**Use:** Searchable choice/typed selection. **Anatomy / exports `[S]`:** Combobox, ComboboxInput, ComboboxContent, ComboboxList, ComboboxItem, ComboboxGroup, ComboboxLabel, ComboboxCollection, ComboboxEmpty, ComboboxSeparator, ComboboxChips, ComboboxChip, ComboboxChipsInput, ComboboxTrigger, ComboboxValue, useComboboxAnchor. **Visual / states:** Base UI-backed; Nova options highlight accent, panel max-h-72, optional chips. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Support typing, Arrow keys, Enter, Escape; label the input; loading/no matches. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Check installed base's item model before shipping; don't blindly copy Radix select syntax.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/combobox.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/combobox) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Combobox items={["Alpha", "Beta"]}><ComboboxInput placeholder="Search" /><ComboboxContent><ComboboxList><ComboboxItem value="Alpha">Alpha</ComboboxItem><ComboboxItem value="Beta">Beta</ComboboxItem></ComboboxList></ComboboxContent></Combobox>
```

#### 6.17. `command`

**Use:** Command launcher and searchable results. **Anatomy / exports `[S]`:** Command, CommandDialog, CommandInput, CommandList, CommandEmpty, CommandGroup, CommandItem, CommandSeparator, CommandShortcut. **Visual / states:** Uses cmdk, dialog, InputGroup; Nova list max-h-72 and highlighted row. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Keyboard navigation, Escape close, shortcut trigger not active while typing. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't invoke unavailable actions or hide empty query state.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/command.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/command) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Command><CommandInput placeholder="Search actions..." /><CommandList><CommandEmpty>No actions found.</CommandEmpty><CommandGroup heading="Projects"><CommandItem>Open project</CommandItem></CommandGroup></CommandList></Command>
```

#### 6.18. `context-menu`

**Use:** Contextual action list on surface. **Anatomy / exports `[S]`:** ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuItem, ContextMenuCheckboxItem, ContextMenuRadioItem, ContextMenuLabel, ContextMenuSeparator, ContextMenuShortcut, ContextMenuGroup, ContextMenuPortal, ContextMenuSub, ContextMenuSubContent, ContextMenuSubTrigger, ContextMenuRadioGroup. **Visual / states:** Nova menu surface with highlighted item; optional destructive state. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Provide visible alternate access on touch/keyboard; keyboard context menu support. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not make right-click the only access path.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/context-menu.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/context-menu) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<ContextMenu><ContextMenuTrigger>Right-click this row</ContextMenuTrigger><ContextMenuContent><ContextMenuItem>Edit</ContextMenuItem></ContextMenuContent></ContextMenu>
```

#### 6.19. `dialog`

**Use:** Modal conversation or editing task. **Anatomy / exports `[S]`:** Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogOverlay, DialogPortal, DialogTitle, DialogTrigger. **Visual / states:** Portal + overlay + content with title/description; Nova motion approx 100ms. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Focus management/restore, Escape, accessible title; keep submit and cancel obvious. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use dialogs for pages requiring complex navigation.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/dialog.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/dialog) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Dialog><DialogTrigger render={<Button />}>Edit</DialogTrigger><DialogContent><DialogHeader><DialogTitle>Edit name</DialogTitle><DialogDescription>Update your display name.</DialogDescription></DialogHeader><Input aria-label="Name" /><DialogFooter><Button>Save</Button></DialogFooter></DialogContent></Dialog>
```

#### 6.20. `drawer`

**Use:** Swipeable sheet-like secondary panel. **Anatomy / exports `[S]`:** Drawer, DrawerPortal, DrawerOverlay, DrawerSwipeHandle, DrawerTrigger, DrawerClose, DrawerContent, DrawerHeader, DrawerFooter, DrawerTitle, DrawerDescription. **Visual / states:** Base-backed drawer (not assumed Vaul); swipe handle and overlay. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Accessible title, dismissal, focus, prevent accidental closing unsaved forms. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not assume legacy Vaul-only props.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/drawer.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/drawer) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Drawer><DrawerTrigger render={<Button variant="outline" />}>Filters</DrawerTrigger><DrawerContent><DrawerHeader><DrawerTitle>Filters</DrawerTitle><DrawerDescription>Refine the results</DrawerDescription></DrawerHeader></DrawerContent></Drawer>
```

#### 6.21. `dropdown-menu`

**Use:** Common action/context menu. **Anatomy / exports `[S]`:** DropdownMenu, DropdownMenuPortal, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuGroup, DropdownMenuLabel, DropdownMenuItem, DropdownMenuCheckboxItem, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSeparator, DropdownMenuShortcut, DropdownMenuSub, DropdownMenuSubTrigger, DropdownMenuSubContent. **Visual / states:** Nova popover, accent highlight, keyboard-first item list. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Arrow, Enter/Space and Escape should work via primitive; state actions clearly. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't place form-like complex content in a menu.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/dropdown-menu.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/dropdown-menu) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<DropdownMenu><DropdownMenuTrigger render={<Button variant="outline" />}>Actions</DropdownMenuTrigger><DropdownMenuContent><DropdownMenuItem>Edit</DropdownMenuItem><DropdownMenuSeparator /><DropdownMenuItem variant="destructive">Delete</DropdownMenuItem></DropdownMenuContent></DropdownMenu>
```

#### 6.22. `empty`

**Use:** Purposeful no-data/failure surface. **Anatomy / exports `[S]`:** Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent, EmptyMedia. **Visual / states:** Semantic title/description/action/optional icon; variant for media. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Differentiate true empty, filtered empty, loading, error, permission denied. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't show “No results” before data has loaded.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/empty.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/empty) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Empty><EmptyHeader><EmptyTitle>No projects</EmptyTitle><EmptyDescription>Create your first project to get started.</EmptyDescription></EmptyHeader><EmptyContent><Button>New project</Button></EmptyContent></Empty>
```

#### 6.23. `field`

**Use:** Composable accessible form layout. **Anatomy / exports `[S]`:** FieldSet, FieldLegend, FieldGroup, Field, FieldContent, FieldLabel, FieldTitle, FieldDescription, FieldSeparator, FieldError. **Visual / states:** Field orientation vertical, horizontal, responsive; FieldError role alert; group container-query responsive. **Exact:** Field `orientation="vertical" | "horizontal" | "responsive"`, FieldError `role="alert"`.

**Keyboard/accessibility & responsive:** Label input and connect description/error IDs via aria-describedby and aria-invalid. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** No placeholder-only labels; don't rely solely on visual proximity.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/field.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/field) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Field><FieldLabel htmlFor="project-name">Project name</FieldLabel><Input id="project-name" /><FieldDescription>Visible to team members.</FieldDescription></Field>
```

#### 6.24. `form`

**Use:** Documented forms integration pattern; not a base registry .tsx file. **Anatomy / exports `[S]`:** Form recipes with Field or form library. **Visual / states:** Use native form + Field components; choose React Hook Form / TanStack Form / Formisch based on app. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use schema validation, per-field errors and focus first invalid control. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not import nonexistent `@/components/ui/form` from this modern base registry without checking project.

**Evidence:** [Base UI registry entry](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts); **no matching installable `.tsx` file** found at inspected path. Do not interpret fragment as a shipped `form`/`toast` module.

```tsx
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
export function EmailForm({ submit }: { submit: (email: string) => Promise<void> }) {
  const [saving, setSaving] = React.useState(false);
  const [error, setError] = React.useState("");
  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setSaving(true); setError("");
    const data = new FormData(e.currentTarget);
    try { await submit(String(data.get("email") ?? "")); }
    catch { setError("Could not submit. Try again."); }
    finally { setSaving(false); }
  }
  return <form onSubmit={handleSubmit} className="space-y-4">
    <Field><FieldLabel htmlFor="email">Email</FieldLabel><Input name="email" id="email" type="email" autoComplete="email" required /></Field>
    {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    <Button type="submit" disabled={saving}>{saving ? "Submitting…" : "Submit"}</Button>
  </form>;
}
```

#### 6.25. `hover-card`

**Use:** Preview supplementary object on hover/focus. **Anatomy / exports `[S]`:** HoverCard, HoverCardTrigger, HoverCardContent. **Visual / states:** Transient popover adjacent to link/trigger. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Keyboard focus must expose content and exit safely; important data visible elsewhere. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use for essential controls on touch.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/hover-card.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/hover-card) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<HoverCard><HoverCardTrigger render={<a href="/users/al">Alice</a>} /><HoverCardContent>Alice — designer</HoverCardContent></HoverCard>
```

#### 6.26. `input`

**Use:** Single-line text/value entry. **Anatomy / exports `[S]`:** Input. **Visual / states:** Input border/input tokens, focus indicator, invalid styling via aria-invalid. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Label, inputmode, autocomplete, errors; never hide focus. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Avoid icon-only placeholders as labels.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/input) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<div className="space-y-2"><Label htmlFor="email">Email</Label><Input id="email" type="email" autoComplete="email" /></div>
```

#### 6.27. `input-group`

**Use:** Input with inline or block addons/actions. **Anatomy / exports `[S]`:** InputGroup, InputGroupAddon, InputGroupButton, InputGroupText, InputGroupInput, InputGroupTextarea. **Visual / states:** Addons at inline-start/end or block positions; compositional input element. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Input must still have label/accessible name; button addon named. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Avoid multiple competing focus outlines and clickable labels.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input-group.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/input-group) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<InputGroup><InputGroupInput aria-label="Search" placeholder="Search..." /><InputGroupAddon align="inline-end"><InputGroupButton aria-label="Clear">×</InputGroupButton></InputGroupAddon></InputGroup>
```

#### 6.28. `input-otp`

**Use:** One-time code entry. **Anatomy / exports `[S]`:** InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator. **Visual / states:** Slots are visual UI for OTP library. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Support paste, numeric keypad, errors and auto-complete one-time-code. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't create six unrelated `<input>` elements.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/input-otp.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/input-otp) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<InputOTP maxLength={6}><InputOTPGroup>{Array.from({length: 6}, (_, i) => <InputOTPSlot key={i} index={i} />)}</InputOTPGroup></InputOTP>
```

#### 6.29. `item`

**Use:** Standard clickable/nonclickable list row. **Anatomy / exports `[S]`:** Item, ItemMedia, ItemContent, ItemActions, ItemGroup, ItemSeparator, ItemTitle, ItemDescription, ItemHeader, ItemFooter. **Visual / states:** Nova variants default, outline, muted; sizes default, sm, xs; leading media/trailing action. **Verified style classes:** variant default/outline/muted; size default/sm/xs.

**Keyboard/accessibility & responsive:** Link and button rows must use semantic interactive element; keep secondary text legible. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't nest button inside clickable button.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/item.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/item) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Item variant="outline"><ItemContent><ItemTitle>API tokens</ItemTitle><ItemDescription>Manage service access</ItemDescription></ItemContent><ItemActions><Button variant="outline" size="sm">Manage</Button></ItemActions></Item>
```

#### 6.30. `label`

**Use:** Accessible control caption. **Anatomy / exports `[S]`:** Label. **Visual / states:** Small high-contrast caption; Base primitive. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use `htmlFor` with id or correct wrapping, required/error signs. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** No independent label without target.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/label.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/label) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Label htmlFor="title">Title</Label>
```

#### 6.31. `menubar`

**Use:** Desktop-style action navigation menus. **Anatomy / exports `[S]`:** Menubar, MenubarPortal, MenubarMenu, MenubarTrigger, MenubarContent, MenubarGroup, MenubarSeparator, MenubarLabel, MenubarItem, MenubarShortcut, MenubarCheckboxItem, MenubarRadioGroup, MenubarRadioItem, MenubarSub, MenubarSubTrigger, MenubarSubContent. **Visual / states:** Menu hierarchy; menu items highlight on focus. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Arrow key traversal, roles and nested submenus per primitive. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not primary website navigation; use NavigationMenu.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/menubar.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/menubar) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Menubar><MenubarMenu><MenubarTrigger>File</MenubarTrigger><MenubarContent><MenubarItem>New</MenubarItem></MenubarContent></MenubarMenu></Menubar>
```

#### 6.32. `navigation-menu`

**Use:** Site/global navigation with disclosures. **Anatomy / exports `[S]`:** NavigationMenu, NavigationMenuContent, NavigationMenuIndicator, NavigationMenuItem, NavigationMenuLink, NavigationMenuList, NavigationMenuTrigger, NavigationMenuPositioner, navigationMenuTriggerStyle. **Visual / states:** Top-level trigger/list/content and positioner. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Link semantics, focus and hover, mobile alternative. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not a replacement for application Sidebar.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/navigation-menu.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/navigation-menu) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<NavigationMenu><NavigationMenuList><NavigationMenuItem><NavigationMenuLink href="/docs">Docs</NavigationMenuLink></NavigationMenuItem></NavigationMenuList></NavigationMenu>
```

#### 6.33. `pagination`

**Use:** Paged dataset navigation. **Anatomy / exports `[S]`:** Pagination, PaginationContent, PaginationEllipsis, PaginationItem, PaginationLink, PaginationNext, PaginationPrevious. **Visual / states:** Button-like anchor controls and current page. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use accessible current-page, disabled first/last logic, stable URL where possible. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not fabricate href destinations.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/pagination.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/pagination) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Pagination><PaginationContent><PaginationItem><PaginationPrevious href="?page=1" /></PaginationItem><PaginationItem><PaginationLink href="?page=2" isActive>2</PaginationLink></PaginationItem><PaginationItem><PaginationNext href="?page=3" /></PaginationItem></PaginationContent></Pagination>
```

#### 6.34. `popover`

**Use:** Nonmodal anchored interactive surface. **Anatomy / exports `[S]`:** Popover, PopoverContent, PopoverDescription, PopoverHeader, PopoverTitle, PopoverTrigger. **Visual / states:** Nova popover surface; content defaults to bottom. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Maintain trigger state, dismiss outside/Escape, label fields. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Use Dialog for attention-critical multi-step flow.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/popover.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/popover) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Popover><PopoverTrigger render={<Button variant="outline" />}>Options</PopoverTrigger><PopoverContent><PopoverHeader><PopoverTitle>Options</PopoverTitle><PopoverDescription>Adjust the view</PopoverDescription></PopoverHeader></PopoverContent></Popover>
```

#### 6.35. `progress`

**Use:** Quantified task completion. **Anatomy / exports `[S]`:** Progress, ProgressTrack, ProgressIndicator, ProgressLabel, ProgressValue. **Visual / states:** Track/indicator separate in modern Base source. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Expose determinate percentage or indeterminate correctly, no unsupported fake accuracy. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Never show 100% before operation completes.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/progress.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/progress) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Progress value={55}><ProgressTrack><ProgressIndicator /></ProgressTrack></Progress>
```

#### 6.36. `radio-group`

**Use:** Choose one from a small discrete set. **Anatomy / exports `[S]`:** RadioGroup, RadioGroupItem. **Visual / states:** Single selection radio primitive. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Arrow keys within group, fieldset/legend or accessible group label. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use checkboxes for mutually exclusive selections.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/radio-group.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/radio-group) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<RadioGroup defaultValue="weekly"><div className="flex items-center gap-2"><RadioGroupItem value="weekly" id="weekly" /><Label htmlFor="weekly">Weekly</Label></div></RadioGroup>
```

#### 6.37. `resizable`

**Use:** Adjustable multi-panel split view. **Anatomy / exports `[S]`:** ResizablePanelGroup, ResizablePanel, ResizableHandle. **Visual / states:** Uses react-resizable-panels; horizontal or vertical orientation. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Keyboard-operable handle, minimum sizes, responsive stacked fallback. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not use resizable for forms on small screens.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/resizable.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/resizable) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<ResizablePanelGroup orientation="horizontal"><ResizablePanel defaultSize={40}>Files</ResizablePanel><ResizableHandle withHandle /><ResizablePanel defaultSize={60}>Preview</ResizablePanel></ResizablePanelGroup>
```

#### 6.38. `scroll-area`

**Use:** Themed scrollable viewport. **Anatomy / exports `[S]`:** ScrollArea, ScrollBar. **Visual / states:** Native scrolling behavior with styled scrollbars. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Preserve wheel, keyboard, screen reader access and visible overflow. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't scroll-lock the full page unnecessarily.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/scroll-area.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/scroll-area) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
export function ScrollableNames({ items }: { items: { id: string; name: string }[] }) {
  return <ScrollArea className="h-72">
    <ul className="space-y-2">{items.map(x => <li key={x.id}>{x.name}</li>)}</ul>
    <ScrollBar orientation="vertical" />
  </ScrollArea>;
}
```

#### 6.39. `select`

**Use:** Single-value dropdown selection. **Anatomy / exports `[S]`:** Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectSeparator, SelectScrollUpButton, SelectScrollDownButton. **Visual / states:** Base UI select (not native); trigger has default/small size. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Label, keyboard selection and selected value; loading/empty states. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Avoid mixing Base UI and Radix Select props.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/select.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/select) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Select defaultValue="draft"><SelectTrigger aria-label="Status"><SelectValue /></SelectTrigger><SelectContent><SelectItem value="draft">Draft</SelectItem><SelectItem value="live">Live</SelectItem></SelectContent></Select>
```

#### 6.40. `separator`

**Use:** Visual or semantic separation. **Anatomy / exports `[S]`:** Separator. **Visual / states:** Border token / horizontal default. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Mark decorative separator decorative; don't replace heading hierarchy. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not place a divider after every row.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/separator.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/separator) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Separator className="my-4" />
```

#### 6.41. `sheet`

**Use:** Responsive slide-over navigation or editor. **Anatomy / exports `[S]`:** Sheet, SheetTrigger, SheetClose, SheetContent, SheetHeader, SheetFooter, SheetTitle, SheetDescription. **Visual / states:** Side defaults right; overlay and portal. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Focus management, dismiss, title, scroll action region. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not overload one sheet with many unrelated tasks.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sheet.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/sheet) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Sheet><SheetTrigger render={<Button variant="outline" />}>Menu</SheetTrigger><SheetContent side="right"><SheetHeader><SheetTitle>Navigation</SheetTitle><SheetDescription>Explore sections</SheetDescription></SheetHeader></SheetContent></Sheet>
```

#### 6.42. `sidebar`

**Use:** Application navigation system. **Anatomy / exports `[S]`:** SidebarProvider, Sidebar, SidebarHeader, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupLabel, SidebarGroupContent, SidebarGroupAction, SidebarInput, SidebarInset, SidebarMenu, SidebarMenuItem, SidebarMenuButton, SidebarMenuAction, SidebarMenuBadge, SidebarMenuSkeleton, SidebarMenuSub, SidebarMenuSubItem, SidebarMenuSubButton, SidebarRail, SidebarSeparator, SidebarTrigger, useSidebar. **Visual / states:** 16rem desktop, 18rem mobile, 3rem icon collapsed; Cmd/Ctrl+B; offcanvas/icon/none, sidebar/floating/inset. **Exact:** Sidebar `side="left" | "right"`, `variant="sidebar" | "floating" | "inset"`, `collapsible="offcanvas" | "icon" | "none"`.

**Keyboard/accessibility & responsive:** Navigation links and selected `isActive` correctly; mobile sheet accessible. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't build a second independent mobile sidebar.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sidebar.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/sidebar) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<SidebarProvider><Sidebar><SidebarHeader>Workspace</SidebarHeader><SidebarContent><SidebarGroup><SidebarGroupLabel>General</SidebarGroupLabel><SidebarGroupContent><SidebarMenu><SidebarMenuItem><SidebarMenuButton isActive>Overview</SidebarMenuButton></SidebarMenuItem></SidebarMenu></SidebarGroupContent></SidebarGroup></SidebarContent></Sidebar><SidebarInset><SidebarTrigger />Content</SidebarInset></SidebarProvider>
```

#### 6.43. `skeleton`

**Use:** Loading placeholder mirroring layout. **Anatomy / exports `[S]`:** Skeleton. **Visual / states:** Muted shimmering placeholder shape; no layout jump. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use busy indicators outside repeated skeleton as needed; avoid presenting skeleton as final data. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't show while data is known empty or failed.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/skeleton.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/skeleton) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Skeleton className="h-6 w-40 rounded-md" />
```

#### 6.44. `slider`

**Use:** Bounded numeric range input. **Anatomy / exports `[S]`:** Slider. **Visual / states:** Radix/Base/Aria-specific primitive; dedicated handle/track internally. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Accessible label, visible value and units, keyboard step; touch control target. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Pass an array of numbers for a single thumb (`defaultValue={[50]}`); the inspected Base wrapper derives thumbs from array length, with a `[min,max]` fallback.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/slider.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/slider) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<div className="space-y-2"><Label id="volume-label">Volume</Label><Slider aria-labelledby="volume-label" defaultValue={[50]} max={100} step={5} /></div>
```

#### 6.45. `sonner`

**Use:** Transient toast announcements. **Anatomy / exports `[S]`:** Toaster. **Visual / states:** Toaster wrapper around Sonner, style-aware. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Success/error updates announced without moving focus; persistent action available for critical failures. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use deprecated toast registry for new implementations.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sonner.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/sonner) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Toaster />
```

#### 6.46. `spinner`

**Use:** In-progress activity ornament. **Anatomy / exports `[S]`:** Spinner. **Visual / states:** Small animation with code-supported size styling. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Pair spinner with adjacent textual status and aria-busy where relevant. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't present spinner forever without timeout/error recovery.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/spinner.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/spinner) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Button disabled><Spinner />Saving…</Button>
```

#### 6.47. `switch`

**Use:** Immediately change binary setting. **Anatomy / exports `[S]`:** Switch. **Visual / states:** `size` default or small in source; thumb and focus state. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Label for setting, Space toggles; async save rollback on error. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use switches for submitting a multi-field form.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/switch.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/switch) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<div className="flex items-center gap-2"><Switch id="updates" /><Label htmlFor="updates">Email updates</Label></div>
```

#### 6.48. `table`

**Use:** Semantic tabular information. **Anatomy / exports `[S]`:** Table, TableHeader, TableBody, TableFooter, TableHead, TableRow, TableCell, TableCaption. **Visual / states:** Structural table, header/body/rows/cells; wrapped horizontally. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Real `<th>` scopes, caption, keyboard sorting controls as buttons; scrolling at small width. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use div grids for true relational data.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/table.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/table) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Table><TableHeader><TableRow><TableHead>Name</TableHead></TableRow></TableHeader><TableBody><TableRow><TableCell>Alpha</TableCell></TableRow></TableBody></Table>
```

#### 6.49. `tabs`

**Use:** Switch between peer views. **Anatomy / exports `[S]`:** Tabs, TabsList, TabsTrigger, TabsContent, tabsListVariants. **Visual / states:** TabsList variants source uses `default` plus others per installed code; selected trigger. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Arrow keys, programmatic active tab, panels; preserve content state intentionally. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not a substitute for permanent routed navigation.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/tabs.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/tabs) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Tabs defaultValue="all"><TabsList><TabsTrigger value="all">All</TabsTrigger><TabsTrigger value="mine">Mine</TabsTrigger></TabsList><TabsContent value="all">All records</TabsContent><TabsContent value="mine">My records</TabsContent></Tabs>
```

#### 6.50. `textarea`

**Use:** Long-form multiline text. **Anatomy / exports `[S]`:** Textarea. **Visual / states:** Input/border/placeholder focus styles. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Clear label, character guidance and errors; resize intentionally. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use one-line Input for lengthy text.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/textarea.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/textarea) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<div className="space-y-2"><Label htmlFor="details">Details</Label><Textarea id="details" rows={4} /></div>
```

#### 6.51. `toast`

**Use:** Deprecated legacy registry entry (no new Base UI implementation file). **Anatomy / exports `[S]`:** Use Sonner Toaster instead. **Visual / states:** Registry entry not a new standard toast component. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Follow Sonner documentation and global toast provider. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not install/use old toast APIs from memory.

**Evidence:** [Base UI registry entry](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts); **no matching installable `.tsx` file** found at inspected path. Do not interpret fragment as a shipped `form`/`toast` module.

```tsx
<Toaster />
```

#### 6.52. `toggle`

**Use:** One independent pressed/unpressed action. **Anatomy / exports `[S]`:** Toggle, toggleVariants. **Visual / states:** Source default/outline style and sizes default/sm/lg subject to version. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Use aria-pressed where primitive handles it, accessible icon-only label. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Not suitable for irreversible submit.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/toggle.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/toggle) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Toggle aria-label="Bold"><BoldIcon className="size-4" /></Toggle>
```

#### 6.53. `toggle-group`

**Use:** Mutually related pressed options. **Anatomy / exports `[S]`:** ToggleGroup, ToggleGroupItem. **Visual / states:** Group variant and size forwarded, multiple/single depends on primitive API. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** State selected labels and allow keyboard choice. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Validate value shape for installed Base UI version; don't assume Radix API.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/toggle-group.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/toggle-group) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<ToggleGroup defaultValue={["grid"]} aria-label="View"><ToggleGroupItem value="grid">Grid</ToggleGroupItem><ToggleGroupItem value="list">List</ToggleGroupItem></ToggleGroup>
```

#### 6.54. `tooltip`

**Use:** Short nonessential explanation for a control. **Anatomy / exports `[S]`:** TooltipProvider, Tooltip, TooltipTrigger, TooltipContent. **Visual / states:** Nova floating arrow/content style; default content above trigger. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Appears on keyboard focus, avoid hiding vital info on touch. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Tooltip must not be the only accessible label.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/tooltip.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/tooltip) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Tooltip><TooltipTrigger render={<Button size="icon" variant="ghost" aria-label="Help" />}>?</TooltipTrigger><TooltipContent>Help</TooltipContent></Tooltip>
```

#### 6.55. `kbd`

**Use:** Keyboard shortcut label. **Anatomy / exports `[S]`:** Kbd, KbdGroup. **Visual / states:** Muted compact mono key styling. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Explain shortcuts through visible UI; don't rely exclusively on them. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not mislabel shortcuts on Windows.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/kbd.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/kbd) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<KbdGroup><Kbd>⌘</Kbd><Kbd>K</Kbd></KbdGroup>
```

#### 6.56. `native-select`

**Use:** Native OS choice list. **Anatomy / exports `[S]`:** NativeSelect, NativeSelectOptGroup, NativeSelectOption. **Visual / states:** Default/small sizes in source; native browser behavior. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Best when browser-standard keyboard, many options, simple forms. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't use Radix/Base Select complexity without need.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/native-select.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/native-select) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<NativeSelect aria-label="Time zone"><NativeSelectOption value="utc">UTC</NativeSelectOption></NativeSelect>
```

#### 6.57. `direction`

**Use:** Direction provider / RTL support. **Anatomy / exports `[S]`:** DirectionProvider, useDirection. **Visual / states:** Exposes direction context for relevant primitives. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Verify RTL layout using logical Tailwind properties and keyboard behavior. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not flip icons/text indiscriminately; not all symbols reverse.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/direction.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/direction) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<DirectionProvider direction="rtl">{/* localized subtree */}</DirectionProvider>
```

#### 6.58. `attachment`

**Use:** Upload/download file surface. **Anatomy / exports `[S]`:** Attachment, AttachmentGroup, AttachmentMedia, AttachmentContent, AttachmentTitle, AttachmentDescription, AttachmentActions, AttachmentAction, AttachmentTrigger. **Visual / states:** Sizes default/sm/xs; orientations horizontal/vertical; states idle/uploading/processing/error/done; media icon/image. **Exact:** Attachment `state="idle" | "uploading" | "processing" | "error" | "done"`, `size="default" | "sm" | "xs"`, `orientation="horizontal" | "vertical"`; AttachmentMedia variant icon/image.

**Keyboard/accessibility & responsive:** Action labels, file size/type, upload status, error recovery, accessible removal. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Never treat client-selected file as uploaded before server success.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/attachment.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/attachment) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Attachment state="done"><AttachmentMedia>PDF</AttachmentMedia><AttachmentContent><AttachmentTitle>report.pdf</AttachmentTitle><AttachmentDescription>Uploaded</AttachmentDescription></AttachmentContent></Attachment>
```

#### 6.59. `bubble`

**Use:** Chat bubble and reactions. **Anatomy / exports `[S]`:** BubbleGroup, Bubble, BubbleContent, BubbleReactions. **Visual / states:** Variant default and alignment start/end; bubble visual grouping. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Speaker context and reaction names; screen readers read in temporal order. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Don't rely on opposite bubble colors as only speaker identification.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/bubble.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/bubble) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Bubble align="start"><BubbleContent>How can I help?</BubbleContent></Bubble>
```

#### 6.60. `message-scroller`

**Use:** Scrollable message collection. **Anatomy / exports `[S]`:** MessageScrollerProvider, MessageScroller, MessageScrollerViewport, MessageScrollerContent, MessageScrollerItem, MessageScrollerButton, useMessageScroller, useMessageScrollerScrollable, useMessageScrollerVisibility. **Visual / states:** Scroll affordances, conversation viewport. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** New message announcements and jump-to-latest control, no forced scroll when reading. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Never hijack text selection or scrolling.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/message-scroller.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/message-scroller) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
import { MessageScrollerProvider, MessageScroller, MessageScrollerViewport, MessageScrollerContent, MessageScrollerItem } from "@/components/ui/message-scroller";
export function ChatTranscript({ messages }: { messages: { id: string; text: string }[] }) {
  return <MessageScrollerProvider>
    <MessageScroller><MessageScrollerViewport>
      <MessageScrollerContent>{messages.map(m => <MessageScrollerItem key={m.id}>{m.text}</MessageScrollerItem>)}</MessageScrollerContent>
    </MessageScrollerViewport></MessageScroller>
  </MessageScrollerProvider>;
}
```

#### 6.61. `questionnaire`

**Use:** Guided AI/form questions. **Anatomy / exports `[S]`:** Questionnaire, QuestionnaireActions, QuestionnaireChoice, QuestionnaireChoiceDescription, QuestionnaireChoices, QuestionnaireDescription, QuestionnaireError, QuestionnaireInput, QuestionnaireItem, QuestionnaireNext, QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip, QuestionnaireSubmit, QuestionnaireTitle. **Visual / states:** Progress, items, choices and actions. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Step focus, keyboard selection, validation and back navigation. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not infer validation API from component names.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/questionnaire.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/questionnaire) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Questionnaire>{/* Explicitly compose documented step content and actions */}</Questionnaire>
```

#### 6.62. `marker`

**Use:** Inline marker. **Anatomy / exports `[S]`:** Marker, MarkerIcon, MarkerContent, markerVariants. **Visual / states:** Accent/status label with icon optional. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Describe meaning in words. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Avoid unreadable decorative status dots.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/marker.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/marker) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Marker><MarkerContent>New</MarkerContent></Marker>
```

#### 6.63. `trace`

**Use:** Expandable AI reasoning/tool trace surface. **Anatomy / exports `[S]`:** Trace, TraceLine, TraceIcon, TraceTitle, TraceDescription, TraceCollapsible, TraceCollapsibleTrigger, TraceCollapsibleContent. **Visual / states:** Trace lines and collapsible details. **Variants:** Do not assume extra variant or size props without opening the installed implementation.

**Keyboard/accessibility & responsive:** Clearly label machine-generated steps and progress; no leaking secrets in trace. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Do not show private chain-of-thought or credential payloads.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/trace.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/trace) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Trace><TraceLine><TraceIcon>•</TraceIcon><TraceTitle>Searching files</TraceTitle><TraceDescription>3 matches</TraceDescription></TraceLine></Trace>
```

#### 6.64. `code-block`

**Use:** Readable/copyable code output. **Anatomy / exports `[S]`:** CodeBlock, CodeBlockHeader, CodeBlockTitle, CodeBlockActions, CodeBlockCopyButton, CodeBlockContent, CodeBlockLine, CodeBlockFooter. **Visual / states:** Variants default/destructive; sizes default/sm; line variants default/added/removed/highlighted, wrap, lineNumbers, startLine. **Exact:** CodeBlock variant default/destructive, `size="default" | "sm"`; line variant default/added/removed/highlighted; copy `value` required.

**Keyboard/accessibility & responsive:** Keyboard-scrollable pre, copy success live region, language indicated textually. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Never style HTML as code without escaped/semantic content.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/code-block.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/code-block) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<CodeBlock><CodeBlockHeader><CodeBlockTitle>config.json</CodeBlockTitle></CodeBlockHeader><CodeBlockContent lineNumbers>{'{"ok": true}'}</CodeBlockContent></CodeBlock>
```

#### 6.65. `message`

**Use:** Chat message layout. **Anatomy / exports `[S]`:** MessageGroup, Message, MessageAvatar, MessageContent, MessageFooter, MessageHeader. **Visual / states:** Align start/end; avatar, header, footer. **Exact:** `align="start" | "end"`.

**Keyboard/accessibility & responsive:** Semantic speaker identity, stable streaming live region. On compact viewports, keep the trigger within the viewport, provide sufficient hit area, and avoid clipping important content. For loading/error/success, use external state management and accessible text rather than undocumented variants. **Do not:** Avoid duplicating raw model output as untrusted HTML.

**Evidence:** [pinned source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/message.tsx) · [Base UI docs](https://ui.shadcn.com/docs/components/base/message) · [Nova CSS](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

```tsx
<Message align="start"><MessageContent><MessageHeader>Assistant</MessageHeader>Ready</MessageContent></Message>
```

### Supplemental recipes (not extra primitives)

The docs navigation includes **Date Picker**, **Data Table**, **Typography** and **AI Elements** recipes beyond the Base UI registry names; they are not counted as additional `registry:ui` components. Date Picker composes `Popover + Calendar + Button` (or native date input when appropriate); Data Table composes `Table + @tanstack/react-table` for sorting/filtering/pagination/selection; Typography uses ordinary semantic HTML + Tailwind classes; AI Elements provide higher-level chat/agent patterns and may have additional package dependencies. The modern index includes `message`, `message-scroller`, `attachment`, `bubble`, `questionnaire`, `trace`, `marker` and `code-block`, so older “shadcn has only ~50 components” summaries are not current for this commit. Sources: [catalog](https://ui.shadcn.com/docs/components), [Data Table](https://ui.shadcn.com/docs/components/data-table), [Date Picker](https://ui.shadcn.com/docs/components/date-picker), [AI Elements](https://ui.shadcn.com/docs/ai-elements), [base index](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/_registry.ts).

**Base comparison:** The Radix index has 64 items and React Aria index has 62, with implementation-specific exclusions (for example, `toast` is represented as deprecated in legacy sources). A feature is **not portable by import-name alone**: component anatomy, `render`/`asChild`, `value` shape and state selectors may differ. Use matching [Radix](https://ui.shadcn.com/docs/components/radix/button) or [Aria](https://ui.shadcn.com/docs/components/aria/button) docs and inspect installed files before switching bases.

## 7. Composition recipes and state machines

### 7.1 Action hierarchy `[R]`

| User intent | Preferred component | Supporting behavior |
|---|---|---|
| Commit current page/form | `Button variant="default"` | One emphasized action per logical action cluster; loading text; validation |
| Cancel/secondary | `Button variant="outline"` or `secondary` | Should not compete with primary CTA |
| Low-impact row icon | `Button variant="ghost" size="icon"` | Accessible name and larger touch target |
| Destructive confirmation | `AlertDialog` | Explain consequence, safe cancel, explicit confirm |
| Open task-specific subview | `Dialog` or `Sheet` | Dialog for focused task, Sheet for lateral details/mobile navigation |
| Choose from small known set | `RadioGroup` / `NativeSelect` | Prefer native when search/custom content unnecessary |
| Choose from long/filterable set | `Combobox` | Keyboard search, no-results state |
| More actions | `DropdownMenu` | Use for secondary commands, not only way to access primary task |
| Global jump | `CommandDialog` | Cmd/Ctrl+K and visible search entry |
| Transient status | `sonner` `Toaster` | Separate from inline persistent error |

### 7.2 Async state model `[R]`

Every data-bearing section explicitly models `idle → loading → success | empty | error`; when mutations occur: `idle → submitting → success | error`. Preserve stale previous data when appropriate instead of tearing down the whole page for background refresh. Render only one primary visual state at a time, with accessible context.

```tsx
// Example status union; prevents contradictory booleans.
type LoadState<T> =
  | { status: "idle" }
  | { status: "loading"; previous?: T }
  | { status: "success"; data: T; updatedAt: string }
  | { status: "empty" }
  | { status: "error"; message: string; previous?: T };

function ProjectStatus({ state, retry }: {
  state: LoadState<{ id: string; name: string }[]>;
  retry: () => void;
}) {
  switch (state.status) {
    case "idle": return null;
    case "loading": return <div aria-busy="true" aria-label="Loading projects" className="space-y-3"><Skeleton className="h-10 w-full" /><Skeleton className="h-10 w-full" /></div>;
    case "empty": return <Empty><EmptyHeader><EmptyTitle>No projects yet</EmptyTitle><EmptyDescription>Start with a new project.</EmptyDescription></EmptyHeader></Empty>;
    case "error": return <Alert role="alert"><AlertTitle>Unable to load projects</AlertTitle><AlertDescription>{state.message}</AlertDescription><AlertAction><Button variant="outline" onClick={retry}>Retry</Button></AlertAction></Alert>;
    case "success": return <ul className="space-y-2">{state.data.map(p => <li key={p.id}>{p.name}</li>)}</ul>;
  }
}
```

**State requirements:** pending save disables duplicate submission and communicates *what* is pending; success stays visible long enough to be perceived, preferably persistent on high-stakes forms; error includes a recovery path. Use optimistic updates only if rollback/error compensation is implemented. A `toast.success()` is not proof of a server write: emit it after confirmed persistence or explain optimistic status.

### 7.3 Data table architecture `[R,D]`

For simple static data, use semantic `Table`. For interactive enterprise tables use `@tanstack/react-table` with controlled state, stable row IDs and explicit `getCoreRowModel`, `getSortedRowModel`, `getFilteredRowModel` and/or server-side mode as appropriate. The shadcn **Data Table is a recipe**, not a new guaranteed `DataTable` primitive. Do not enable both server and client pagination accidentally; avoid using array index as row key. Make column sorting a `<button>` inside `<TableHead>`, exposing sorting direction via `aria-sort` on the `<th>` where practical. Keep selection checkboxes labeled with row entity names; batch toolbar appears only when selection exists.

```tsx
// Domain contract; see §12.4 for a complete interactive example.
type ListQuery = {
  search: string;
  sort: { key: "name" | "createdAt"; descending: boolean };
  page: number;
  pageSize: 25 | 50 | 100;
};
// Controlled state belongs at the page/controller level, not inside a generic TableCell.
```

### 7.4 Forms and validation `[R]`

Prefer native constraints for simple forms, then add Zod + React Hook Form/TanStack Form for complex data and async validation. The modern Base UI registry's `form` entry does not establish an installed `Form` module; **use `Field`, `FieldLabel`, `FieldDescription`, `FieldError`, `FieldGroup`** or locally installed equivalents. Give each input stable `id`, `name`, autocomplete when relevant, error text `id`, `aria-invalid`, and `aria-describedby`; preserve entered values after failure; only reset on confirmed success. Show a summary and focus first invalid field on failed submit if several controls error. Never validate solely through color.

### 7.5 Application-specific compositions `[R]`

**Search:** `FieldLabel` + `InputGroup` search icon + debounced async query + keyboard-accessible suggestions (`Command` or `Combobox`) + distinct “no matches” and network errors. Keep query in URL when useful. Cancel stale requests using `AbortController`; do not show a previous query's results as current.

**CRUD:** Query listing; Create via `Dialog` with form and optimistic guard; Edit via action in `DropdownMenu`; Delete via `AlertDialog`; after server response, refresh/invalidate cache. Errors stay inline on the affected row as well as a toast where useful. Preserve keyboard focus to trigger after closing.

**File manager:** Breadcrumb, toolbar, `Table` or `ItemGroup`, file icons/media, upload via `Attachment` with state, context menu with visible fallback. Provide list/grid choice, upload progress, errors, empty folder vs. filter no matches, and access control display. Enforce actual backend permission; UI visibility alone is not authorization.

**Chat:** `MessageScrollerProvider` + `Message` (role name, timestamp) + composer (`Textarea` / `InputGroup`) + `Attachment` + `CodeBlock` for code. Support `Enter` to send only by explicit product policy (Shift+Enter newline), `Stop generating`, regenerate/retry, streaming partial content, keyboard transcript access, and scroll-to-bottom affordance. Avoid autoplay voice/animation.

**Onboarding:** one step per screen; `Questionnaire` may be used after verifying its API; otherwise use `FieldSet`/`FieldGroup` with clear progress and Back/Next. Persist drafts safely, label optional steps, surface validation before moving on, summarize settings at completion.

**Notification center:** `Popover` for short recent list, `ItemGroup` for items, visible unread count with text equivalent, `Empty` state. Use Sonner for transactional toasts; do not use toasts as the only record of important events.

**Analytics:** Card title, timeframe/filter, clearly labeled chart and data table, timezone, comparison window and units. Avoid default chart colors without mapping labels. Include a meaningful zero-data explanation; distinguish zero, missing and suppressed values.

## 8. Interaction, keyboard, and motion standards

### 8.1 Universal interaction states `[R; some classes source-backed]`

| State | Required visible behavior | Semantic behavior | Anti-pattern |
|---|---|---|---|
| Default | Clear affordance for interaction | Proper role/name | Everything looks clickable |
| Hover | Subtle accent/color change only where pointer exists | No reliance on hover alone | Hover shows critical hidden control |
| Focus-visible | High contrast focus ring/border with offset as needed | Keyboard navigable | `outline-none` without replacement |
| Pressed/active | Brief feedback, avoid layout shift | `aria-pressed` for toggle, active nav state | `translate` on focus/selected changes layout |
| Selected | Selected indicator plus text/state | aria-selected/checked/current as appropriate | Color-only selection |
| Disabled | Perceived unavailability and contextual reason | Native disabled or truthful aria-disabled | Disabled action with no explanation |
| Loading | Spinner + changing verb, stable width | `aria-busy` / status text | Flickering infinite spinner |
| Invalid | Border/error text, recovery guidance | `aria-invalid`, linked error | Red border alone |
| Success | Persistent or reliably announced confirmation | polite live region as appropriate | Success before server confirmation |
| Reduced motion | Static transition / little or no movement | `prefers-reduced-motion` | Mandatory animated reveal |

**Inspected Nova evidence:** Button root includes `disabled:pointer-events-none disabled:opacity-50`, `focus-visible:ring-3` via `.cn-button`, with `ring-ring/50`; destructive focus changes ring; popover-like components use open/closed fade and zoom classes with `duration-100`; `.cn-command-item` highlights selected via `bg-muted`. Source [Button](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/button.tsx) and [Nova](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/styles/style-nova.css).

### 8.2 Keyboard contract `[R]`

| Pattern | Expected keyboard workflow |
|---|---|
| Button | Tab to focus, Enter/Space activate |
| Checkbox / Switch | Tab to focus, Space toggle |
| Radio Group | Tab into group, arrow keys select/move when primitive supports convention |
| Tabs | Arrow-key tab movement, Tab enters active panel |
| Menu/Combobox/Command | Arrow Up/Down, Enter selection, Escape close, typeahead/search |
| Dialog/AlertDialog | Focus moved into content, Tab stays in modal, Escape if dismissal allowed, focus returns to opener |
| Tooltip/HoverCard | Trigger focus accessible, Escape dismissal where applicable |
| Sidebar | Cmd/Ctrl+B toggles per inspected source; users still have visible SidebarTrigger |
| Table sorting | Focus header's button, activate, announce sort direction |
| File uploads | Keyboard-accessible browse/clear/retry; drag-and-drop is additive, not sole method |
| Chat composer | Documented Enter vs Shift+Enter; stop/cancel accessible; streaming not a focus trap |

**Global shortcuts `[R]`:** never steal Cmd/Ctrl+K or Cmd/Ctrl+B from active editors/inputs where it conflicts. Remove keydown listeners on unmount, respect `event.defaultPrevented`, `event.isComposing`, `input`/`textarea`/`contentEditable`, and disable while modal editing unless the shortcut is intentionally scoped.

### 8.3 Motion and feedback `[R]`

Use CSS transitions for color/opacity/transform only; avoid `transition-all` on large trees, width/height layout animation in data-dense views, and parallax. Follow installed components' built-in open/close transition before writing extra animation. Touch devices do not have hover; actions needed for use must remain visible. Honor reduced motion:

```css
/* Additional project recommendation; not a shadcn upstream token. */
@media (prefers-reduced-motion: reduce) {
  .app-motion {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

Prefer targeted `motion-reduce:animate-none` on actual animated elements. **Do not** globally suppress all focus rings, scroll behavior, or genuine progress status.

## 9. WCAG 2.2 AA and test plan

This section is a **project compliance target `[R]`**; neither source-distributed components nor a primitive library makes a complete product WCAG compliant automatically. Reference: [WCAG 2.2 W3C Recommendation](https://www.w3.org/TR/WCAG22/).

| Requirement | Practical acceptance criteria | Responsibility |
|---|---|---|
| 1.1.1 Non-text Content | All meaningful images/icons have accessible names or text alternatives | App author |
| 1.3.1 Info and Relationships | Real headings, labels, tables, lists, fieldsets and structure | App author |
| 1.4.3 Contrast (minimum) | Normal text at least 4.5:1; large text at least 3:1, subject to WCAG exceptions | App theme + author |
| 1.4.11 Non-text Contrast | UI boundaries/important icons/focus indicators generally at least 3:1 where required | Theme + author |
| 1.4.10 Reflow | Content usable at 320 CSS pixels equivalent without two-dimensional scrolling, except allowed exceptions such as data tables | Author |
| 1.4.12 Text Spacing | UI survives increased line/paragraph/word/letter spacing without loss | Author |
| 2.1.1 Keyboard | Every control usable without mouse; drag alternatives | Primitive + author |
| 2.1.2 No Keyboard Trap | Focus can leave every region; dialogs dismiss appropriately | Primitive + author |
| 2.4.3 Focus Order | Logical DOM/keyboard progression | Author |
| 2.4.7 Focus Visible | Visible focus indication | Primitive CSS + theme |
| 2.4.11 Focus Not Obscured (Minimum) | Focusable target not entirely hidden behind sticky overlays | Author |
| 2.5.7 Dragging Movements | Provide single-pointer alternative to dragging | Author |
| 2.5.8 Target Size (Minimum) | 24×24 CSS px or qualifying exception; recommended larger on touch | Author |
| 3.3.1 Error Identification | Explain which input failed and why | Author |
| 3.3.2 Labels or Instructions | Stable visible labels, requirements, guidance | Author |
| 3.3.3 Error Suggestion | Suggested correction when known | Author |
| 3.3.7 Redundant Entry | Avoid re-entering previously provided information where applicable | Product + author |
| 4.1.2 Name, Role, Value | Custom controls expose correct semantics | Primitive + author |
| 4.1.3 Status Messages | Async status announced without disruptive focus move | Author |

**Built-in vs required:** Base UI, Radix and React Aria primitives provide valuable focus, roles and keyboard interaction mechanisms for their respective components **when used correctly**; they do **not** automatically provide good labels, readable theme contrast, validation messages, accessible chart summaries, safe tab order across custom page layouts or your application's domain status announcements. Source copy is yours to maintain.

**Mandatory validation sequence `[R]`:** (1) static lint/typecheck, (2) `@axe-core/playwright` / axe automated scans for every representative route and dark/light mode, (3) `eslint-plugin-jsx-a11y` where compatible, (4) keyboard-only walkthrough with visible focus and no mouse, (5) screen-reader smoke test with VoiceOver on macOS/iOS and NVDA on Windows for key flows, (6) 200% and 400% zoom/reflow, (7) forced-colors/high-contrast where supported, (8) prefers-reduced-motion test. Automated axe passing is **necessary but not sufficient**.

**A11y test cases to automate:** dialog opens focused and returns to trigger; menu arrows and Escape; form invalid input links error; table sorting updates label; responsive sheet can be dismissed; no scroll lock on modal close; toast has status text; icon-only control named; keyboard-only file upload; no invisible focus under header. Use the actual framework-specific test runner and installed component base.

```ts
// Playwright example (requires @axe-core/playwright in dev dependencies)
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("settings is navigable and has no detectable axe violations", async ({ page }) => {
  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).analyze();
  expect(accessibility.violations).toEqual([]);
});
```

## 10. Themes, icons, imagery, and visual quality

### 10.1 Theme governance `[D,R]`

The official theming architecture uses CSS variables within `:root` and `.dark`, exposed through `@theme inline` to Tailwind utility names. The `components.json` **project configuration** determines style, base, aliases and color treatment. For a new project, initialize through the CLI rather than copying components manually. Example based on [official theming docs](https://ui.shadcn.com/docs/theming):

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "base-nova",
  "rsc": true,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "app/globals.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "utils": "@/lib/utils",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide"
}
```

**Install workflow `[D]`:** `pnpm dlx shadcn@latest init` then `pnpm dlx shadcn@latest add button card dialog field input table sidebar sonner` after confirming chosen style/base. `shadcn docs`, `shadcn view` and `shadcn info` can inspect installed behavior. CLI flags change over time; pin CLI and dependencies in `pnpm-lock.yaml` for reproducibility. This `components.json` is a *recommended app-specific sample*, **not** the repo's own `apps/v4/components.json` (which points to legacy `new-york`). If customizing tokens, keep contrast audit in all modes.

Theme handling in Next.js normally uses `next-themes` with root `class="dark"` and `attribute="class"`, with system preference and SSR hydration precautions. [Dark mode docs](https://ui.shadcn.com/docs/dark-mode/next). Avoid a theme toggle that flashes the wrong theme or has an inaccessible icon-only name.

```tsx
"use client";
import { ThemeProvider as NextThemesProvider } from "next-themes";
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return <NextThemesProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange>{children}</NextThemesProvider>;
}
```

**Branding `[R]`:** override *semantic token pairs*, not all `.cn-*` selectors. Add `--success` and `--warning` only if the product needs them, with foreground partners, and validate both modes. Prefer container-level theme scopes for a branded section with local overrides; never mix Sera and Nova style classes arbitrarily. Third-party embedded widgets require separate contrast review. Use RTL migration/docs for languages with right-to-left writing systems.

### 10.2 Iconography and images `[R]`

Use `lucide-react` within a product unless the initialized preset intentionally chooses another library. In inspected source, icon placeholders accommodate multiple icon packs for documentation previews; that **does not** require app users to install all of them. Typical sizes: 12px small badge, 14px compact metadata, 16px form/toolbar, 20px navigation emphasis, 24px standalone. In Nova default Button, unsized SVG is 16px; preserve icon alignment `shrink-0`, and `aria-hidden="true"` when label text already supplies meaning. Icon-only buttons need `aria-label` describing action (“More options for Project Alpha”, not “three dots”).

Image recipes: `Avatar` uses square crop/round mask, cards with hero imagery use `aspect-video` and `object-cover`, screenshots prefer `object-contain` to preserve UI detail, profile photos should have appropriate alt and safe fallback. Don't use generic decorative 3D graphics in functional tables/forms. A blank-state illustration may complement `Empty`, but must not obscure the primary recovery action. Optimize images (Next Image where applicable), specify dimensions/aspect ratio to prevent layout shift, honor privacy when images are uploaded by users.

### 10.3 Pixel-quality review `[R]`

A visually finished page has: one readable H1 and clear supporting text; semantic neutral surfaces; equal left edges; consistent header heights and button size; related controls on the same baseline; stable text wrapping; one canonical border and radius language; visually quiet separators; no truncation of action names or error text; consistent hover/focus treatment; dark theme with comparable information hierarchy; mobile layout designed, not merely squeezed; informative skeleton and empty states; no dead buttons; exact icon alignment. Screenshot at all test viewports and inspect before merging. Fix spacing with layout primitives rather than arbitrary `top-[3px]`, `translate-x-1.5`, or new hard-coded hex values.

## 11. Frontend architecture and AI agent contract

### 11.1 Component ownership and code organization `[R]`

```text
src/
  app/                # routes, server data boundaries, page/layout composition
  components/
    ui/               # CLI-owned shadcn source; narrow, documented changes only
    layout/           # AppSidebar, PageHeader, ContentFrame
    data-display/     # MetricCard, StatusBadge, EmptyData, DataTable
    forms/            # branded domain fields, validated forms
    features/         # project / account / billing domain compositions
  hooks/              # side-effect and accessibility-oriented hooks
  lib/                # validation, formatting, typed API clients
  styles/             # theme tokens and intentional app-wide additions
```

TypeScript: strict mode, derive props from existing components (`React.ComponentProps<typeof Button>`), use domain discriminated unions rather than many contradictory booleans, don't redefine native keyboard semantics, validate external data on trust boundaries. Use proper keys, stable actions, `AbortController`, controlled component state only when needed, lazy load expensive charts/code editors, debounce network search, paginate/virtualize large tables when performance requires, avoid stateful charts rendering on every keystroke. Never install a second UI framework just to implement one feature already composed from present primitives.

### 11.2 Mandatory sequence for AI coding agents `[R]`

1. **Inspect:** read `components.json`, package/lockfiles, `globals.css`, actual `components/ui` files, existing pages and screenshot/style conventions. Record selected base/style, Tailwind version, icon library, state/data libraries.
2. **Specify:** write a 4–8 sentence page spec: user goal, content hierarchy, actions, data states, empty/error flow, breakpoints, accessibility and sources of truth.
3. **Reuse:** map each interaction to the component decision table (§7.1); install missing primitives through CLI consistent with project, not by pasting older API snippets.
4. **Layout:** construct semantic skeleton, mobile-first page gutter, max-width, grid, header and sidebar; ensure `min-w-0` and overflow boundaries.
5. **Theme:** use semantic tokens and active style; don't introduce arbitrary palette/radius or speculative props.
6. **Implement:** keyboard/labels/focus/errors *during* implementation, not in a cleanup step; use strict types and state unions.
7. **Validate:** typecheck, lint, test, functional interactions, axe, keyboard, dark/light, 320px/768px/1440px, zoom, reduced motion.
8. **Inspect diff:** no unrelated file changes, unused dependencies, duplicate component shells, hard-coded status colors, or unsupported API use.
9. **Deliver:** report which files changed, test commands/results, accessibility findings, remaining risks, and any recommendations diverging from source.

### 11.3 DO / DON'T lint-like policy `[R]`

| Don't | Do | Reason |
|---|---|---|
| `className="bg-blue-600 text-white"` for primary action | `<Button>Save</Button>` | Token/theme coherence |
| `<div onClick={save}>Save</div>` | `<Button type="button" onClick={save}>Save</Button>` | Keyboard and semantics |
| `<Button asChild>` with Base UI | Use installed Base UI button's `render` / documented props, or native link | Base API differs from legacy |
| Every panel `shadow-xl rounded-3xl` | `Card` with style-owned radius/ring | Aesthetic consistency |
| `text-xs text-gray-400` for all labels | `FieldLabel`, `text-sm font-medium` | Readability and semantics |
| `outline-none` on controls | Installed `focus-visible` styles, or tested ring | Visible keyboard focus |
| `isLoading && isError && !data` boolean pile | Discriminated `LoadState<T>` | Contradiction prevention |
| Toast on every keystroke | Validate on blur/submit and use inline status | Less interruption |
| Hide table columns unannounced | Responsive scroll or deliberate column-priority disclosure | Preserve data context |
| `dark:bg-black` overrides everywhere | Semantic `bg-background`, `bg-card` | Consistent dark mode |
| `z-[999999]` everywhere | Use portal/overlay stack policy | Manageable layering |
| Skeleton when API returned empty | `Empty` with next action | Truthful state |
| A separate icon pack per page | Single approved icon system | Consistency and bundle size |
| Generate chart labels from colors alone | Series names, units and accessible text/table | Interpretability |
| Hard-code `100vh` for mobile | `min-h-svh` or appropriate dynamic viewport primitive | Browser chrome handling |

### 11.4 Required pre-/post-implementation checklists `[R]`

**Before coding:**

- [ ] Project version and primitive base established; no Base/Radix/Aria API confusion.
- [ ] Current components catalog and installed wrappers inspected; variant names taken from source.
- [ ] Page goal, key action, data sources, error/empty/loading conditions defined.
- [ ] Layout specified at 320, 768, 1440px; explicit overflow/scroll plan.
- [ ] Semantic page and form hierarchy defined; interaction keyboard map created.
- [ ] Existing tokens/colors and style selected; no unreviewed design forks.

**After coding:**

- [ ] All visible controls work, no inert CTA, placeholder links, or dead menu items.
- [ ] Browser typecheck/lint/build/test run with results recorded.
- [ ] Axe + manual keyboard + at least one screen-reader journey checked.
- [ ] Focus visible and restored after modal/menu dismissal; errors announced.
- [ ] No light/dark contrast failures; icon-only elements named.
- [ ] 320px/768px/1440px and 200–400% zoom without avoidable clipping.
- [ ] Loading/empty/error/success/retry states shown in product.
- [ ] Reduced-motion and touch behavior checked.
- [ ] Visual screenshot comparison: alignment, 4px rhythm, type, buttons, borders.
- [ ] Source compatibility, dependencies, and diff checked; no unrelated modifications.

## 12. Ten reference implementations

These are application-level **reference snippets `[R]`**, intentionally plain, tokenized and composable. They demonstrate layout and interaction patterns, not a new official library of shadcn components. They assume `base-nova`, installed components from the named imports, React 19/TypeScript, Tailwind v4, and the necessary upstream packages as declared in the registry. Add real data fetching, authorization, server validation, localization and tests in the adopting project. No example should be represented as executed/compiled in this audit; the upstream monorepo was not runnable in this environment. Prefer individual imports from the installed version and make API adjustments after `shadcn view <component>`.

### 12.1 SaaS dashboard (semantic hierarchy + metrics + activity)

```tsx
import * as React from "react";
import { Button } from "@/components/ui/button";
import {
  Card, CardContent, CardDescription, CardHeader, CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ArrowUpRight, Plus } from "lucide-react";

type Metric = { id: string; label: string; value: string; delta: string; summary: string };
const METRICS: Metric[] = [
  { id: "revenue", label: "Revenue", value: "$48,320", delta: "+12.4%", summary: "vs. previous month" },
  { id: "users", label: "Active users", value: "8,412", delta: "+6.1%", summary: "past 30 days" },
  { id: "conv", label: "Conversion", value: "4.8%", delta: "+0.4 pt", summary: "vs. previous month" },
  { id: "tickets", label: "Open tickets", value: "27", delta: "−8", summary: "vs. previous week" },
];
function MetricCard({ metric }: { metric: Metric }) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-3">
        <CardDescription>{metric.label}</CardDescription>
        <Badge variant="outline">{metric.delta}</Badge>
      </CardHeader>
      <CardContent className="space-y-1">
        <p className="text-2xl font-semibold tracking-tight tabular-nums">{metric.value}</p>
        <p className="text-xs text-muted-foreground">{metric.summary}</p>
      </CardContent>
    </Card>
  );
}
export function OverviewPage({ onNewReport, performanceChart }: {
  onNewReport: () => void;
  performanceChart: React.ReactNode;
}) {
  return (
    <main className="mx-auto w-full max-w-7xl space-y-6 px-4 py-6 sm:px-6 lg:px-8">
      <header className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
          <p className="mt-1 text-sm text-muted-foreground">A summary of business performance.</p>
        </div>
        <Button type="button" onClick={onNewReport}><Plus className="size-4" aria-hidden="true" /> New report</Button>
      </header>
      <section aria-label="Key performance indicators" className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {METRICS.map(metric => <MetricCard key={metric.id} metric={metric} />)}
      </section>
      <div className="grid gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(18rem,1fr)]">
        <Card><CardHeader><CardTitle>Performance</CardTitle>
          <CardDescription>Daily performance over time</CardDescription></CardHeader>
          <CardContent>{performanceChart}</CardContent>
        </Card>
        <Card><CardHeader><CardTitle>Recent activity</CardTitle></CardHeader>
          <CardContent><ul className="divide-y divide-border">
            {["Report created", "Workspace updated", "Invite accepted"].map(x => (
              <li key={x} className="flex items-center justify-between gap-2 py-3 text-sm">
                <span>{x}</span><ArrowUpRight className="size-4 text-muted-foreground" aria-hidden="true" />
              </li>
            ))}</ul></CardContent>
        </Card>
      </div>
    </main>
  );
}
```

**Agent checks:** Key metrics use text and contextual deltas, never red/green alone. Supply the accessible chart from §12.3 through `performanceChart`, backed by real data. `onNewReport` is a required callback rather than a dead button. Add time window, data freshness and real data fetch in product code.

### 12.2 Sidebar application shell (source-specific widths and mobile Sheet)

```tsx
"use client";
import {
  Sidebar, SidebarContent, SidebarFooter, SidebarGroup,
  SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarInset,
  SidebarMenu, SidebarMenuButton, SidebarMenuItem, SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { Folder, House, Settings } from "lucide-react";

const NAV = [
  { label: "Overview", href: "/dashboard", icon: House },
  { label: "Projects", href: "/projects", icon: Folder },
  { label: "Settings", href: "/settings", icon: Settings },
] as const;

export function ApplicationShell({ children, pathname }: {
  children: React.ReactNode;
  pathname: string;
}) {
  return (
    <SidebarProvider defaultOpen>
      <Sidebar collapsible="icon" variant="sidebar">
        <SidebarHeader className="px-3 py-3 text-sm font-semibold">Acme Console</SidebarHeader>
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>Workspace</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {NAV.map(item => (
                  <SidebarMenuItem key={item.href}>
                    <SidebarMenuButton
                      render={<a href={item.href} />}
                      isActive={pathname === item.href}
                      tooltip={item.label}
                    >
                      <item.icon className="size-4" aria-hidden="true" />
                      <span>{item.label}</span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>
        <SidebarFooter className="px-3 py-3 text-xs text-sidebar-foreground">Acme workspace</SidebarFooter>
      </Sidebar>
      <SidebarInset className="min-w-0">
        <header className="flex h-14 items-center gap-3 border-b px-4 sm:px-6">
          <SidebarTrigger aria-label="Toggle sidebar" />
          <Separator orientation="vertical" className="h-5" />
          <span className="truncate text-sm font-medium">Workspace</span>
        </header>
        <div className="min-w-0 flex-1">{children}</div>
      </SidebarInset>
    </SidebarProvider>
  );
}
```

**Verified metrics:** 16rem desktop width, 18rem Sheet mobile, 3rem icon mode; Cmd/Ctrl+B. Source: [Sidebar implementation](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/sidebar.tsx). Verify installed `SidebarMenuButton` render/tooltip type before copying into a different primitive base. A responsive sidebar must remain navigable with screen reader and keyboard.

### 12.3 Analytics card and accessible Recharts chart

```tsx
"use client";
import { Line, LineChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "@/components/ui/card";
import { ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
import type { ChartConfig } from "@/components/ui/chart";

type Point = { date: string; sessions: number };
const CHART_CONFIG = {
  sessions: { label: "Sessions", color: "var(--chart-1)" },
} satisfies ChartConfig;
export function SessionsChart({ data }: { data: Point[] }) {
  const total = data.reduce((sum, point) => sum + point.sessions, 0);
  return (
    <Card>
      <CardHeader><CardTitle>Sessions over time</CardTitle>
        <CardDescription>Daily sessions; values in sessions per day</CardDescription></CardHeader>
      <CardContent>
        {data.length === 0 ? (
          <p role="status" className="py-12 text-center text-sm text-muted-foreground">No sessions available in this period.</p>
        ) : (
          <>
            <p className="sr-only">Total sessions over {data.length} days: {total.toLocaleString()}.</p>
            <ChartContainer config={CHART_CONFIG} className="h-64 w-full min-w-0">
              <LineChart accessibilityLayer data={data} margin={{ left: 8, right: 8, top: 8, bottom: 0 }}>
                <CartesianGrid vertical={false} strokeDasharray="3 3" />
                <XAxis dataKey="date" tickLine={false} axisLine={false} />
                <YAxis width={36} tickLine={false} axisLine={false} />
                <ChartTooltip content={<ChartTooltipContent />} />
                <Line type="monotone" dataKey="sessions" stroke="var(--color-sessions)" strokeWidth={2} dot={false} />
              </LineChart>
            </ChartContainer>
            <details className="mt-4 text-sm"><summary className="cursor-pointer text-muted-foreground">View values</summary>
              <table className="mt-2 w-full text-left"><caption className="sr-only">Daily sessions</caption>
                <thead><tr><th scope="col">Date</th><th scope="col">Sessions</th></tr></thead>
                <tbody>{data.map((row, index) => <tr key={`${row.date}-${index}`}><td>{row.date}</td><td>{row.sessions}</td></tr>)}</tbody>
              </table>
            </details>
          </>
        )}
      </CardContent>
    </Card>
  );
}
```

**Scope:** This uses the registry's Recharts wrapper; inspected Base registry specifies `recharts@3.8.0` at this commit. The table is intentionally offered as an alternate representation. Production must use real keys, format axis labels with locale/timezone and test line/chart contrast in both modes. Source: [chart.tsx](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/chart.tsx).

### 12.4 Data management screen (search, sort, row actions, confirmation)

```tsx
"use client";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent,
  AlertDialogDescription, AlertDialogFooter, AlertDialogHeader,
  AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/alert-dialog";

type Project = { id: string; name: string; owner: string; status: "Active" | "Draft" };
function DeleteProjectAction({ project, onRemove }: {
  project: Project;
  onRemove: (id: string) => Promise<void>;
}) {
  const [open, setOpen] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      await onRemove(project.id);
      setOpen(false); // Close only when the server confirmed deletion.
    } catch {
      setError("Could not delete the project. Your changes were not applied.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <AlertDialog open={open} onOpenChange={next => { if (!busy) setOpen(next); }}>
      <AlertDialogTrigger render={<Button variant="ghost" size="sm" disabled={busy} />}>
        Delete
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {project.name}?</AlertDialogTitle>
          <AlertDialogDescription>This permanently removes the project and cannot be undone.</AlertDialogDescription>
        </AlertDialogHeader>
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>Cancel</AlertDialogCancel>
          <AlertDialogAction type="button" variant="destructive" disabled={busy} onClick={() => void confirm()}>
            {busy ? "Deleting…" : "Delete project"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function ProjectManager({ projects, onRemove, onCreate }: {
  projects: Project[];
  onRemove: (id: string) => Promise<void>;
  onCreate: () => void;
}) {
  const [query, setQuery] = React.useState("");
  const [ascending, setAscending] = React.useState(true);
  const results = React.useMemo(() => projects
    .filter(p => `${p.name} ${p.owner}`.toLowerCase().includes(query.trim().toLowerCase()))
    .sort((a, b) => ascending ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name)),
    [projects, query, ascending]);
  return (
    <section className="mx-auto max-w-6xl space-y-4 px-4 py-6 sm:px-6" aria-labelledby="projects-title">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 id="projects-title" className="text-2xl font-semibold tracking-tight">Projects</h1>
          <p className="text-sm text-muted-foreground">Manage workspace projects</p>
        </div>
        <Button type="button" onClick={onCreate}>Create project</Button>
      </div>
      <label htmlFor="project-search" className="sr-only">Search projects</label>
      <Input id="project-search" value={query} onChange={e => setQuery(e.target.value)}
        placeholder="Search by project or owner" className="max-w-sm" />
      <div className="min-w-0 overflow-x-auto rounded-lg border">
        <Table>
          <TableHeader><TableRow>
            <TableHead aria-sort={ascending ? "ascending" : "descending"}>
              <button type="button" className="font-medium underline-offset-4 hover:underline focus-visible:rounded-sm focus-visible:ring-2 focus-visible:ring-ring"
                onClick={() => setAscending(a => !a)}>Name {ascending ? "↑" : "↓"}</button>
            </TableHead>
            <TableHead>Owner</TableHead>
            <TableHead>Status</TableHead>
            <TableHead className="text-right">Actions</TableHead>
          </TableRow></TableHeader>
          <TableBody>
            {results.length === 0 ? <TableRow><TableCell colSpan={4} className="py-12 text-center text-muted-foreground">
              {projects.length === 0 ? "No projects yet." : "No matches for these filters."}
            </TableCell></TableRow> : results.map(p => (
              <TableRow key={p.id}>
                <TableCell className="font-medium">{p.name}</TableCell>
                <TableCell>{p.owner}</TableCell>
                <TableCell><Badge variant={p.status === "Active" ? "secondary" : "outline"}>{p.status}</Badge></TableCell>
                <TableCell className="text-right"><DeleteProjectAction project={p} onRemove={onRemove} /></TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}
```

**Integration details:** Inspected Base UI `AlertDialogAction` is a styled `Button`, **not an automatic dismiss-on-success operation**. The extracted row action uses controlled `open/onOpenChange`, closes only after `onRemove` resolves, and shows an inline error while preserving the dialog on failure. The required `onCreate` callback must open a creation form or navigate to it. Real backends must authorize each mutation and update the owning projects state after success. Source: [Base alert dialog source](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/alert-dialog.tsx).

### 12.5 Settings page (controlled preferences and save feedback)

```tsx
"use client";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";

type Preferences = { displayName: string; updates: boolean };
export function PreferencesPage({ initial, save }: {
  initial: Preferences;
  save: (next: Preferences) => Promise<void>;
}) {
  const [value, setValue] = React.useState(initial);
  const [savedBaseline, setSavedBaseline] = React.useState(initial);
  const [status, setStatus] = React.useState<"idle" | "saving" | "saved" | "error">("idle");
  React.useEffect(() => { setSavedBaseline(initial); setValue(initial); }, [initial]);
  const dirty = value.displayName !== savedBaseline.displayName || value.updates !== savedBaseline.updates;
  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!dirty || status === "saving") return;
    setStatus("saving");
    try { await save(value); setSavedBaseline(value); setStatus("saved"); }
    catch { setStatus("error"); }
  }
  return (
    <main className="mx-auto w-full max-w-3xl space-y-6 px-4 py-6">
      <div><h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-muted-foreground">Your workspace preferences.</p></div>
      <Card><CardHeader><CardTitle>Profile & notifications</CardTitle>
        <CardDescription>Changes apply to your account.</CardDescription></CardHeader>
        <CardContent>
          <form onSubmit={submit} className="space-y-6">
            <FieldGroup>
              <Field><FieldLabel htmlFor="display-name">Display name</FieldLabel>
                <Input id="display-name" autoComplete="name" required value={value.displayName}
                  onChange={e => { setValue(v => ({ ...v, displayName: e.target.value })); setStatus("idle"); }} />
                <FieldDescription>Shown to your collaborators.</FieldDescription>
              </Field>
              <Field orientation="horizontal" className="justify-between">
                <div><FieldLabel htmlFor="notifications">Email notifications</FieldLabel>
                  <FieldDescription>Receive activity summaries.</FieldDescription></div>
                <Switch id="notifications" checked={value.updates}
                  onCheckedChange={checked => { setValue(v => ({ ...v, updates: checked })); setStatus("idle"); }} />
              </Field>
            </FieldGroup>
            <div className="flex items-center gap-3">
              <Button type="submit" disabled={!dirty || status === "saving"}>
                {status === "saving" ? "Saving…" : "Save changes"}
              </Button>
              <p role="status" className="text-sm text-muted-foreground">
                {status === "saved" ? "Settings saved" : status === "error" ? "Save failed. Please retry." : ""}
              </p>
            </div>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
```

**Review:** Saving successfully updates the confirmed baseline and disables Save until another change. The inspected `Switch` wrapper forwards Base UI `Root.Props`; `onCheckedChange` is a Base UI event (verify the installed dependency type after updating packages). Decide how to merge server-pushed `initial` updates with any locally unsaved edits before adopting this reference in a multi-user application.

### 12.6 Authentication page (accessible form and recoverable error)

```tsx
"use client";
import * as React from "react";
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";

export function SignIn({ authenticate }: {
  authenticate: (email: string, password: string) => Promise<void>;
}) {
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const data = new FormData(e.currentTarget);
    const email = String(data.get("email") ?? "");
    const password = String(data.get("password") ?? "");
    setBusy(true); setError(null);
    try { await authenticate(email, password); }
    catch { setError("Sign-in failed. Check your credentials and try again."); }
    finally { setBusy(false); }
  }
  return (
    <main className="grid min-h-svh place-items-center px-4 py-10">
      <Card className="w-full max-w-sm">
        <CardHeader><CardTitle>Sign in</CardTitle>
          <CardDescription>Use your work account to continue.</CardDescription></CardHeader>
        <CardContent><form onSubmit={submit} className="space-y-5" aria-busy={busy}>
          <FieldGroup>
            <Field><FieldLabel htmlFor="auth-email">Email</FieldLabel>
              <Input name="email" id="auth-email" type="email" autoComplete="email" required autoFocus /></Field>
            <Field><FieldLabel htmlFor="auth-password">Password</FieldLabel>
              <Input name="password" id="auth-password" type="password" autoComplete="current-password" required /></Field>
          </FieldGroup>
          {error && <FieldError id="auth-error">{error}</FieldError>}
          <Button className="w-full" type="submit" disabled={busy}>
            {busy ? <><Spinner /> Signing in…</> : "Sign in"}
          </Button>
          <a href="/forgot-password" className="block text-center text-sm underline underline-offset-4">Forgot password?</a>
        </form></CardContent>
      </Card>
    </main>
  );
}
```

**Security boundary:** real auth must be handled by trusted backend with CSRF/session/cookie and rate-limit controls; never store credentials in analytics, local storage or toast messages. Avoid `autoFocus` on mobile if it causes unwanted keyboard takeover; test. An actual implementation should link `auth-error` to the form/inputs via live announcement and focus management and avoid account enumeration through error details.

### 12.7 Complex validated form (Zod + React Hook Form + Field)

```tsx
"use client";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldError, FieldGroup, FieldLabel, FieldDescription } from "@/components/ui/field";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

const schema = z.object({
  projectName: z.string().min(3, "Use at least 3 characters").max(80),
  contactEmail: z.email("Enter a valid email"),
  description: z.string().max(2000, "Keep description under 2,000 characters"),
});
type Values = z.infer<typeof schema>;

export function ProjectForm({ create }: { create: (values: Values) => Promise<void> }) {
  const { register, handleSubmit, formState: { errors, isSubmitting }, setError } = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { projectName: "", contactEmail: "", description: "" },
  });
  return (
    <Card className="mx-auto w-full max-w-xl">
      <CardHeader><CardTitle>Create a project</CardTitle></CardHeader>
      <CardContent>
        <form noValidate onSubmit={handleSubmit(async values => {
          try { await create(values); }
          catch { setError("root", { message: "Unable to create project. Retry." }); }
        })} className="space-y-6">
          <FieldGroup>
            <Field data-invalid={!!errors.projectName}>
              <FieldLabel htmlFor="project-name">Project name</FieldLabel>
              <Input id="project-name" {...register("projectName")}
                aria-invalid={!!errors.projectName} aria-describedby={errors.projectName ? "name-error" : "name-help"} />
              {errors.projectName ? <FieldError id="name-error">{errors.projectName.message}</FieldError>
                : <FieldDescription id="name-help">3–80 characters.</FieldDescription>}
            </Field>
            <Field data-invalid={!!errors.contactEmail}>
              <FieldLabel htmlFor="contact-email">Contact email</FieldLabel>
              <Input id="contact-email" type="email" autoComplete="email" {...register("contactEmail")}
                aria-invalid={!!errors.contactEmail} aria-describedby={errors.contactEmail ? "email-error" : undefined} />
              {errors.contactEmail && <FieldError id="email-error">{errors.contactEmail.message}</FieldError>}
            </Field>
            <Field data-invalid={!!errors.description}>
              <FieldLabel htmlFor="description">Description</FieldLabel>
              <Textarea id="description" rows={5} {...register("description")}
                aria-invalid={!!errors.description} aria-describedby={errors.description ? "description-error" : "description-help"} />
              {errors.description ? <FieldError id="description-error">{errors.description.message}</FieldError>
                : <FieldDescription id="description-help">Optional. Up to 2,000 characters.</FieldDescription>}
            </Field>
          </FieldGroup>
          {errors.root && <p role="alert" className="text-sm text-destructive">{errors.root.message}</p>}
          <div className="flex justify-end"><Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? "Creating…" : "Create project"}</Button></div>
        </form>
      </CardContent>
    </Card>
  );
}
```

**Dependencies:** `zod`, `react-hook-form`, `@hookform/resolvers` are application-added dependencies, not automatically provided by `field`. `z.email()` follows modern Zod API; adapt for the project-installed Zod version. For production add focus-first-error and server error mapping to the actual fields, and navigate/reset on success. Source: [Field](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/field.tsx).

### 12.8 Command palette (Cmd/Ctrl+K, search and action dispatch)

```tsx
"use client";
import * as React from "react";
import {
  Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput,
  CommandItem, CommandList, CommandShortcut,
} from "@/components/ui/command";
import { Button } from "@/components/ui/button";

export function GlobalCommand({ go }: { go: (path: string) => void }) {
  const [open, setOpen] = React.useState(false);
  React.useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement | null;
      if (e.defaultPrevented || e.isComposing || target?.matches('input, textarea, [contenteditable="true"]')) return;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault(); setOpen(current => !current);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
  const destinations = [
    { label: "Overview", path: "/dashboard" },
    { label: "Projects", path: "/projects" },
    { label: "Settings", path: "/settings" },
  ];
  return <>
    <Button variant="outline" onClick={() => setOpen(true)}>
      Search <span className="ml-3 text-xs text-muted-foreground">Ctrl/⌘ K</span>
    </Button>
    <CommandDialog open={open} onOpenChange={setOpen} title="Command menu"
      description="Search commands and destinations">
      <Command><CommandInput placeholder="Type to search..." />
        <CommandList><CommandEmpty>No matching destinations.</CommandEmpty>
          <CommandGroup heading="Navigate">
            {destinations.map(x => <CommandItem key={x.path} value={x.label}
              onSelect={() => { setOpen(false); go(x.path); }}>{x.label}</CommandItem>)}
          </CommandGroup>
        </CommandList>
      </Command>
    </CommandDialog>
  </>;
}
```

**Source:** [command.tsx](https://github.com/shadcn-ui/ui/blob/96da9cc7f1aa10580a3741664e2a77c721206c00/apps/v4/registry/bases/base/ui/command.tsx) uses `cmdk`. Different keyboard layouts require platform-aware shortcut copy; keyboard shortcut should not preempt focused text editor. Keep a visible clickable trigger. Replace `go()` with framework navigation.

### 12.9 Responsive website navigation (desktop links, mobile Sheet)

```tsx
"use client";
import { Menu } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription, SheetTrigger } from "@/components/ui/sheet";
import { NavigationMenu, NavigationMenuList, NavigationMenuItem, NavigationMenuLink } from "@/components/ui/navigation-menu";

const LINKS = [
  { label: "Product", href: "/product" },
  { label: "Pricing", href: "/pricing" },
  { label: "Docs", href: "/docs" },
] as const;
export function WebsiteNav() {
  return (
    <header className="border-b bg-background">
      <div className="mx-auto flex h-14 max-w-7xl items-center justify-between gap-4 px-4 sm:px-6 lg:px-8">
        <a href="/" className="shrink-0 text-sm font-semibold focus-visible:rounded-sm focus-visible:ring-2 focus-visible:ring-ring">Acme</a>
        <NavigationMenu className="hidden md:flex">
          <NavigationMenuList className="gap-2">
            {LINKS.map(item => <NavigationMenuItem key={item.href}>
              <NavigationMenuLink href={item.href} className="rounded-md px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground">
                {item.label}
              </NavigationMenuLink>
            </NavigationMenuItem>)}
          </NavigationMenuList>
        </NavigationMenu>
        <Sheet>
          <SheetTrigger render={<Button size="icon" variant="outline" aria-label="Open navigation" className="md:hidden" />}>
            <Menu className="size-4" aria-hidden="true" />
          </SheetTrigger>
          <SheetContent side="right">
            <SheetHeader><SheetTitle>Navigation</SheetTitle><SheetDescription>Explore Acme</SheetDescription></SheetHeader>
            <nav aria-label="Mobile navigation" className="mt-6 flex flex-col gap-1">
              {LINKS.map(x => <a key={x.href} href={x.href}
                className="rounded-md px-3 py-3 text-sm font-medium hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring">
                {x.label}
              </a>)}
            </nav>
          </SheetContent>
        </Sheet>
      </div>
    </header>
  );
}
```

**Checks:** Mobile sheet's links should close the sheet if client navigation does not trigger a full page unload; use controlled `open` where needed. `NavigationMenu` is website nav; `Sidebar` is application nav. Avoid two landmarks with the same accessible name.

### 12.10 AI chat interface (text, send state, and extension hooks)

```tsx
"use client";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Message, MessageContent, MessageGroup, MessageHeader } from "@/components/ui/message";
import { Spinner } from "@/components/ui/spinner";
import { Send, Square } from "lucide-react";

type ChatMessage = { id: string; role: "user" | "assistant"; content: string };
export function ChatPanel({ messages, send, stop, generating }: {
  messages: ChatMessage[];
  send: (message: string) => Promise<void>;
  stop: () => void;
  generating: boolean;
}) {
  const [draft, setDraft] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const endRef = React.useRef<HTMLDivElement | null>(null);
  const viewportRef = React.useRef<HTMLDivElement | null>(null);

  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const text = draft.trim();
    if (!text || sending || generating) return;
    setSending(true); setError(null);
    try { await send(text); setDraft(""); }
    catch { setError("Message failed to send. Your text was preserved; retry."); }
    finally { setSending(false); }
  }
  return (
    <section aria-label="Assistant conversation" className="mx-auto flex h-[min(90svh,54rem)] w-full max-w-3xl flex-col overflow-hidden rounded-xl border bg-background">
      <header className="flex h-14 shrink-0 items-center border-b px-4">
        <h1 className="text-base font-semibold">Assistant</h1>
      </header>
      <div ref={viewportRef} className="min-h-0 flex-1 overflow-y-auto px-4 py-6" role="log" aria-label="Conversation" aria-live="polite" aria-relevant="additions text">
        {messages.length === 0 ? <p className="py-16 text-center text-sm text-muted-foreground">Ask a question to get started.</p> : (
          <MessageGroup className="gap-6">
            {messages.map(m => <Message key={m.id} align={m.role === "user" ? "end" : "start"}>
              <MessageContent className={m.role === "user" ? "max-w-[85%] rounded-xl bg-muted p-3" : "max-w-full"}>
                <MessageHeader className="mb-1 text-xs font-medium text-muted-foreground">{m.role === "user" ? "You" : "Assistant"}</MessageHeader>
                <p className="whitespace-pre-wrap text-sm leading-6">{m.content}</p>
              </MessageContent>
            </Message>)}
          </MessageGroup>
        )}
        {generating && <p role="status" className="mt-4 flex items-center gap-2 text-sm text-muted-foreground"><Spinner /> Generating response</p>}
        <div ref={endRef} />
      </div>
      <form onSubmit={submit} className="shrink-0 border-t p-3 sm:p-4">
        <label htmlFor="chat-input" className="sr-only">Message to assistant</label>
        <div className="flex items-end gap-2">
          <Textarea id="chat-input" value={draft} onChange={e => setDraft(e.target.value)}
            placeholder="Ask anything…" rows={2} className="min-h-12 resize-none" />
          {generating ? (
            <Button type="button" variant="outline" onClick={stop} aria-label="Stop generating"><Square className="size-4" /> Stop</Button>
          ) : (
            <Button type="submit" disabled={!draft.trim() || sending} aria-label="Send message">
              {sending ? <Spinner /> : <Send className="size-4" aria-hidden="true" />}
              <span className="hidden sm:inline">Send</span>
            </Button>
          )}
        </div>
        {error && <p role="alert" className="mt-2 text-sm text-destructive">{error}</p>}
        <p className="mt-2 text-xs text-muted-foreground">Your messages are processed according to this app's privacy policy.</p>
      </form>
    </section>
  );
}
```

**Production extensions:** This reference avoids insecure Markdown-to-HTML rendering. For actual assistant output add safe Markdown rendering and sanitized code blocks, scroll only when the user is near bottom, include a scroll-to-latest control (or adopt the verified `MessageScroller` components), manage streaming token announcements to avoid screen-reader spam, implement attachments with `Attachment` state and MIME/size validation, retries and conversation persistence. `role="log"` can become excessively verbose with continuous text streaming; throttle accessible status updates rather than reading each token. A backend must filter or isolate untrusted model/tool-generated text.


## 13. Compliance checklist and verification log

### 13.1 Source-to-document evidence ledger

The authoritative snapshot for this document is commit [`96da9cc7f1aa10580a3741664e2a77c721206c00`](https://github.com/shadcn-ui/ui/commit/96da9cc7f1aa10580a3741664e2a77c721206c00), inspected on **2026-10-29**. The following table is intentionally explicit about what was examined and what remains unverified.

| Area | Evidence inspected | Result / limits |
|---|---|---|
| Revision | GitHub commit search for `shadcn-ui/ui` | Latest commit returned was `96da9cc7…`, commit date 2026-10-27; exact SHA above. A future update must re-pin. |
| Base registry inventory | `apps/v4/registry/bases/base/ui/_registry.ts` | **65** `registry:ui` entries, enumerated exactly in §6. `form` and `toast` are special cases; this is **not** 65 separate usable `.tsx` modules. |
| Radix inventory | `apps/v4/registry/bases/radix/ui/_registry.ts` | **64** entries, a comparison target, not a pixel-equivalent substitute. |
| React Aria inventory | `apps/v4/registry/bases/aria/ui/_registry.ts` | **62** entries, a comparison target with distinct APIs. |
| Registry pipeline | `apps/v4/registry/README.md`, `apps/v4/registry/bases/README.md`, `apps/v4/registry/bases/base/registry.ts` | Authored vs generated directory distinction; `index`/`style` dependencies; base/style combinations. |
| Legacy migration | `apps/v4/registry/new-york-v4/ui/button.tsx` and its UI index | Verified legacy `asChild` vs current Base UI `ButtonPrimitive.Props`; do not cross-copy. |
| Component implementation | Base `ui/*.tsx` wrappers across the inventory; deeper read of button, card, field, sidebar, command, chart, dialog, alert-dialog, code-block, attachment, message, slider, resizable, toggle-group | Named exports, selected explicit options, structure, and styles. Individual primitive-library props and every nested branch were **not** independently runtime-tested. |
| Style metrics | `apps/v4/registry/styles/style-nova.css`, comparative `style-sera.css`, `style-vega.css` | Nova CSS classes and explicit sizes; do not generalize across styles. Full cascade and runtime build output not executed. |
| Theme scaffold | [Official theming documentation](https://ui.shadcn.com/docs/theming) | Exact neutral color table and CSS foundation in §3; custom scheme values are **project recommendations**, not upstream constants. |
| Components, CLI, install | [Catalog](https://ui.shadcn.com/docs/components), [CLI](https://ui.shadcn.com/docs/cli), [Next install](https://ui.shadcn.com/docs/installation/next) | Component/documentation relationship; CLI installs copied source. Other frameworks were not audited end-to-end. |
| Accessibility | [WCAG 2.2](https://www.w3.org/TR/WCAG22/), primitive wrappers, selected descriptions | Target and project test plan, **not** a certified accessibility audit of a running app. |
| Block/page patterns | Inspected architecture, samples, registry and documented usage | Ten purpose-built reference screen snippets are new project examples `[R]`; **not** copied/snapshotted upstream blocks, and not evidence that the source supports every app-domain action. |

### 13.2 Enforceable design compliance checklist `[R]`

An AI agent must complete this checklist **for each new page or materially changed screen**. Failing a *blocker* means do not present the screen as production ready.

| Gate | Must be true | Failure severity | How to prove |
|---|---|---|---|
| Source compatibility | Selected primitive base, style and installed exports/props match the actual project | **Blocker** | Check `components.json`, installed file and dependency versions; typecheck |
| Semantic hierarchy | One page-level H1, proper headings, main/navigation landmarks and labeled areas | **Blocker** | DOM inspection + heading/landmark audit |
| Operable controls | Each visible CTA has a real action, state or navigation; no inert decorative button | **Blocker** | Manual interaction or component test |
| Destructive confirmation | Action explains consequences, enables cancel, awaits server result; preserves error on failure | **Blocker** | Integration tests with resolve/reject scenarios |
| Form accessibility | Labels, stable IDs, errors and descriptions associated, busy state, no lost input on failure | **Blocker** | Keyboard + axe + submission/error tests |
| Keyboard | All interactive elements focusable in sensible order; Enter/Space/arrows/Escape where appropriate | **Blocker** | Keyboard-only walkthrough |
| Focus | Visible, not obscured; trap/restore behavior of active modal and menu is validated | **Blocker** | E2E focus assertions |
| Colors and theme | Only semantic tokens in app composition; both light and dark tested | **Blocker** | Search arbitrary palette overrides; screenshot, contrast tool |
| Reflow | Main tasks work at 320 CSS px and at large desktop; scrolling is intentional | **Blocker** | Playwright viewport tests; 200/400% zoom |
| Data states | Loading, empty, error, success, stale/pending (where relevant) are distinct and truthful | Major | Storybook/state fixture review |
| Layout rhythm | Gutter, max-width, aligned baselines, consistent component sizes and card gaps | Major | Screenshot diff vs approved states |
| Icons and imagery | Single icon pack, accessible icon-only buttons, deliberate crop/alt and no layout shift | Major | A11y and image audit |
| Async reliability | Clear busy state, no double-submit, actionable retry, cancellation/cleanup as needed | **Blocker** for mutations | Integration test; network failure simulation |
| Motion | No necessary content only in animation; reduced-motion honored | Major | Emulated reduced-motion test |
| Performance | Tables/streams do not render unbounded items; charts/editors lazy where warranted | Major | Profiling and representative dataset test |
| Final evidence | Build/types/lint/tests pass; screenshots manually reviewed; diff scoped | **Blocker** | CI output and review note |

### 13.3 Practical QA commands and test inventory `[R]`

Run commands against the **actual consuming project**, not the text-only `design.md` document. Replace package-manager/script names according to that project's lockfile and package scripts. The document's JSX snippets have been **syntax-parsed** but are not claimed to have been built against a live project.

```bash
# Examples; only run scripts that exist in the target package.json.
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm exec playwright test
```

Recommended Playwright/Chromatic/Percy/story screenshot set (choose whichever is already adopted):

| Viewport / configuration | Screens to snapshot | Special failure to look for |
|---|---|---|
| 320×700, light | Dashboard, table, auth, sheet, chat | Horizontal page overflow, hidden actions, fixed-width fields |
| 390×844, dark | Dashboard, settings, menu, confirmation, chat | Contrast, touch targets, browser bottom chrome |
| 768×1024 | Sidebar transition, forms, data management | Duplicate mobile/desktop nav and awkward intermediate columns |
| 1280×800 | Full desktop shell, data grid, command | Misaligned toolbar, unexpected nested scrollbar |
| 1440×900 | Dense and comfortable page modes, chart | Large blank areas, line-length errors, excess elevation |
| 320 CSS px at 400% zoom | All high-priority workflows | Obscured focus, two-dimensional layout scrolling where avoidable |
| Reduced motion / forced colors | Menus, dialogs, sorting, validation | Invisible focus/selection or essential animated instructions |

**Component-level regression matrix:** Button variants (`default`, `outline`, `secondary`, `ghost`, `destructive`, `link`) at their Nova source sizes; form field valid/invalid/disabled; menu keyboard open/select/Escape; dialog dismiss/focus return; sidebar desktop expanded/collapsed and mobile Sheet; long data table with many columns; chart with empty and non-empty data; Date Picker with keyboard; rich-text/code overflow; AI chat streaming/pending/error. If a component name is absent from the chosen base, replace it with a documented composition rather than inventing an export.

### 13.4 Things explicitly *not* established by this audit

1. **No complete executable clone.** The environment could not clone `github.com` from the container because of DNS failure. The source audit used pinned GitHub file retrieval. Generated registry output/build artifacts and every upstream example/block file were not analyzed exhaustively.
2. **No real project root was mounted.** The generated file is delivered at `/mnt/data/design.md` rather than being committed inside an existing user repo; copy or upload it into the target repository's root before using it as agent instructions.
3. **No end-to-end compiled app.** Text-only JSX examples in §6 are composition fragments, and ten substantial examples in §12 are reference implementations; actual TypeScript typechecking, CSS production output, dependency resolution, screen-reader behavior and visual baseline screenshots require the target application.
4. **No universal token claims.** Neutral tokens from the official docs and exact **Nova** metrics are distinct. The document is **not** a precise token specification for every named style, custom theme or registry provider. In particular, typography hierarchy, app container widths, density modes and breakpoints chosen for pages are `[R]` conventions.
5. **No automatic WCAG certification.** Accessibility measures are target rules and tests; supported primitives reduce, but do not remove, application responsibility.
6. **External dependencies and APIs may change.** Re-validate the installed components and compare upstream SHA before significant upgrades; `render` vs `asChild`, event callback shape, chart package and icon pack are high-risk drift points.

### 13.5 Maintenance contract `[R]`

When updating shadcn/ui or changing style/primitive base: (1) record new SHA/date; (2) re-enumerate all three registries and compare to the 65/64/62 baseline; (3) compare authored `style-*.css`, default theme variables, `button`, `field`, `dialog`, `select`, `sidebar`, `chart`, `command`, and AI component exports; (4) update each affected spec and code sample; (5) rerun type/lint/build/a11y/visual tests; (6) add migration notes distinguishing breaking behavior from changes to local `[R]` conventions. Re-audit the design system at every major Tailwind or component-base upgrade, not only when visual regressions appear.

**Definition of done:** A consuming AI agent can identify the chosen primitive/style, use source-backed component names and visual defaults, implement the described application recipes with accessible loading/error/recovery, and verify the resulting UI. This document is the baseline contract; the *installed project code* remains authoritative for APIs and the running application's visual and accessibility tests remain the final acceptance evidence.
