---
name: KittyRed
description: "A calm, dense, local A-share simulation workstation."
colors:
  bg: "#06111a"
  bg-elevated: "#09121df5"
  panel: "#edf3ff0b"
  panel-strong: "#0e1b2af5"
  border: "#edf3ff17"
  text: "#edf3ff"
  text-muted: "#edf3ffb8"
  accent: "#8fdcff"
  accent-strong: "#6cf4cb"
  accent-fill: "#9ad8e3"
  accent-fill-text: "#0b2431"
  positive: "#7ef7c4"
  negative: "#ff8a80"
  success-bg: "#8ee0b2"
  success-text: "#0d3425"
  warning-bg: "#e8c56d"
  warning-text: "#3b2c0d"
  danger-bg: "#c65d43"
  danger-text: "#fff3ef"
  info-bg: "#5e8fd8"
  info-text: "#eef6ff"
  neutral-bg: "#40505f"
  neutral-text: "#dfe7f6"
typography:
  display:
    fontFamily: "SF Pro Display, Segoe UI, sans-serif"
    fontSize: "clamp(1.8rem, 3vw, 2.4rem)"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "normal"
  headline:
    fontFamily: "SF Pro Display, Segoe UI, sans-serif"
    fontSize: "clamp(1.3rem, 2vw, 1.7rem)"
    fontWeight: 700
    lineHeight: 1.2
  title:
    fontFamily: "SF Pro Display, Segoe UI, sans-serif"
    fontSize: "1.15rem"
    fontWeight: 700
    lineHeight: 1.25
  body:
    fontFamily: "SF Pro Display, Segoe UI, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.55
  label:
    fontFamily: "SF Pro Display, Segoe UI, sans-serif"
    fontSize: "0.74rem"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0.1em"
rounded:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  panel: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent-fill}"
    textColor: "{colors.accent-fill-text}"
    rounded: "{rounded.lg}"
    padding: "12px 14px"
    height: "48px"
  button-ghost:
    backgroundColor: "#ffffff14"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "12px 14px"
  panel:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.xl}"
    padding: "24px"
  input:
    backgroundColor: "#ffffff0a"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "12px 14px"
    height: "48px"
  nav-active:
    backgroundColor: "{colors.accent-fill}"
    textColor: "{colors.accent-fill-text}"
    rounded: "{rounded.lg}"
    padding: "10px 12px"
---

# Design System: KittyRed

## 1. Overview

**Creative North Star: "The Quiet Trading Desk"**

KittyRed should feel like a local A-share research desk: calm, compact, and trustworthy enough for repeated scanning. The interface is dark because the user is comparing tables, signals, settings, and assistant context in a focused desktop workflow, not because the product wants a dramatic trading-terminal costume.

The system uses restrained color, dense but readable spacing, and familiar product patterns: fixed desktop sidebar, table shells, field grids, drawers, tabs, chips, and explicit status language. It rejects crypto vocabulary, CEX exchange branding, USDT pricing, 霓虹交易终端, 重渐变 AI 仪表盘, 装饰性玻璃拟态, and any visual treatment that blurs the boundary between local simulation and real trading.

**Key Characteristics:**
- Local simulation first: every trading surface must read as paper/simulated, never brokerage-connected.
- Data dense but ordered: tables, fields, and signal panels should support scanning and comparison.
- Chinese UI by default: user-visible labels, states, errors, buttons, and empty states use Chinese.
- Calm contrast: cyan is a functional accent, not decoration.
- Touch-aware responsive behavior: 44px minimum touch targets and structural breakpoints at 900px and 640px.

## 2. Colors

The palette is a tinted dark workstation with one cool cyan accent and semantic fills for market/status states.

### Primary
- **Workbench Cyan** (#8fdcff): Focus rings, section labels, active navigation support, and subtle system emphasis.
- **Active Cyan Fill** (#9ad8e3): Primary actions, active sidebar links, selected tabs, and selected segmented controls. Use it sparingly so active state stays obvious.
- **Mint Confirmation** (#6cf4cb): Secondary accent for positive emphasis where the main cyan would be ambiguous.

### Secondary
- **Signal Green** (#7ef7c4): Gains, approved states, and positive numeric values.
- **Risk Coral** (#ff8a80): Losses, blocked states, failed states, and error copy.
- **Signal Amber** (#e8c56d): Queued, watch, warning, and medium-risk states.

### Neutral
- **Night Ledger** (#06111a): App background and page base.
- **Raised Rail** (#09121df5): Sidebar and sticky mobile navigation background.
- **Ink Panel** (#edf3ff0b): Default panel/card surface.
- **Strong Drawer** (#0e1b2af5): Assistant drawer, modal, and other overlay surfaces.
- **Fine Divider** (#edf3ff17): Borders, table separators, form strokes, and surface dividers.
- **Paper Text** (#edf3ff): Primary text.
- **Dim Annotation** (#edf3ffb8): Metadata, helper copy, secondary labels, empty-state notes.

### Named Rules
**The Cyan Is State Rule.** Cyan marks actions, focus, active navigation, and selected filters. Do not use it as decorative atmosphere.

**The No Crypto Palette Rule.** Do not introduce neon purple, exchange-brand color palettes, USDT green-black terminal styling, or high-chroma gradients.

## 3. Typography

**Display Font:** SF Pro Display, with Segoe UI and sans-serif fallback  
**Body Font:** SF Pro Display, with Segoe UI and sans-serif fallback  
**Label/Mono Font:** Use the same family; rely on tabular numerals where dense numeric comparison needs it.

**Character:** Native product typography, not editorial. The system should feel like a macOS desktop utility with enough hierarchy to scan, not a landing page with oversized type.

### Hierarchy
- **Display** (700, `clamp(1.8rem, 3vw, 2.4rem)`, 1.15): Page titles only, especially app headers.
- **Headline** (700, `clamp(1.3rem, 2vw, 1.7rem)`, 1.2): Panel titles and hero-panel summaries.
- **Title** (700, `1.15rem`, 1.25): Sidebar brand, compact panel titles, and dense card headings.
- **Body** (400, `1rem`, 1.55): Descriptions, setting explanations, assistant text, and empty states. Keep prose at roughly 62ch when possible.
- **Label** (600, `0.74rem`, `0.1em`, uppercase only for structural labels): Section labels, status metadata, and compact tags.

### Named Rules
**The Product Scale Rule.** Do not use hero-scale typography inside panels, settings tabs, drawers, chips, or buttons. Compact surfaces need compact type.

**The Chinese First Rule.** Do not ship English UI labels where the user sees them. Strategy names, dialog labels, buttons, empty states, and errors should be Chinese.

## 4. Elevation

KittyRed uses tonal layering plus restrained shadows. Panels are separated by fine borders and low-alpha fills; shadows are structural, not decorative. Drawers can cast stronger side shadows because they must separate from the workspace while preserving the dark working surface.

### Shadow Vocabulary
- **Surface Shadow** (`0 14px 38px rgba(1, 7, 13, 0.22)`): Default panel and major surface lift.
- **Side Drawer Shadow** (`-14px 0 42px rgba(0, 0, 0, 0.26)`): Assistant and audit drawers only.
- **Modal Shadow** (`0 18px 48px rgba(0, 0, 0, 0.32)`): Modal content and isolated overlays.

### Named Rules
**The No Glass Rule.** Do not use backdrop blur or 装饰性玻璃拟态 as the shell language. Depth comes from border, tone, and restrained shadow.

**The No Side Stripe Rule.** Do not use thick colored `border-left` or `border-right` accents on cards, callouts, or alerts.

## 5. Components

### Buttons
- **Shape:** Rounded product controls, usually 16px; compact nav controls may use 12px.
- **Primary:** Active cyan fill with dark text, 12px 14px padding, 48px minimum height for form/action surfaces.
- **Hover / Focus:** Hover may shift background subtly; focus always uses a 3px Workbench Cyan outline with 3px offset.
- **Ghost:** White 8% fill on dark surfaces, text in Paper Text, same shape vocabulary as primary actions.

### Chips
- **Style:** Pill chips use 999px radius, 7px 10px padding, semantic token fills, and high-contrast text.
- **State:** Active filters and selected tabs use Active Cyan Fill. Running/approved use success fill, queued uses amber, blocked/failed uses danger.

### Cards / Containers
- **Corner Style:** 20px for general panels and metric cards; 8px only for dense specialized regions like pair-detail hero blocks and charts.
- **Background:** Default panels use Ink Panel, drawers use Strong Drawer, nested tools use low-alpha white fills.
- **Shadow Strategy:** Use Surface Shadow on page panels; avoid stacked/nested card shadows.
- **Border:** 1px Fine Divider is the default separator.
- **Internal Padding:** 24px for panels, 16px for cards/metrics, 12px for compact controls.

### Inputs / Fields
- **Style:** 16px radius, 1px Fine Divider, `rgba(255,255,255,0.04)` fill, 12px 14px padding, 48px minimum height.
- **Focus:** Global 3px cyan focus outline. Do not replace it with only color or shadow.
- **Error / Disabled:** Keep user input visible; use Risk Coral copy for errors and opacity/cursor state for disabled buttons.

### Navigation
- **Desktop:** Fixed 264px left sidebar with vertical navigation, 10px gaps, and active cyan fill.
- **Tablet:** At 900px, the sidebar becomes a normal top section and content margin resets.
- **Phone:** At 640px, the sidebar becomes a sticky compact top navigation. The nav list scrolls horizontally, hint copy is hidden, and the assistant button remains accessible.

### Tables
- **Style:** Dense tables live in `.table-shell`; wide comparison tables use `.table-shell--visible-scrollbar` and explicit minimum widths.
- **Behavior:** Preserve horizontal scroll for dense A-share data rather than wrapping numeric columns.

### Dialogs / Drawers
- **Drawers:** Fixed right side, Strong Drawer background, 100vw width on phones, and `aria-modal="true"` when modal.
- **Modals:** Centered overlay with 60% black scrim, 12px radius, 20px padding, and max-height scrolling.

## 6. Do's and Don'ts

### Do:
- **Do** keep the interface cold, professional, and data dense, like a local A-share research workstation.
- **Do** use Workbench Cyan for active controls, selected navigation, and visible focus states.
- **Do** keep all user-visible UI copy in Chinese.
- **Do** preserve simulation language: "模拟账户", "本地模拟", "仅模拟账号模式".
- **Do** use horizontal scroll shells for wide financial tables.
- **Do** keep touch targets at 44px minimum and form controls around 48px high.
- **Do** use structural breakpoints at 900px and 640px before trying fluid type tricks.
- **Do** use drawers or overlays for deep detail views when in-flow expansion would shift page weight.

### Don't:
- **Don't** use crypto vocabulary, CEX exchange brands, USDT计价, or real-brokerage account language.
- **Don't** make the product feel like a 霓虹交易终端, a crypto exchange, or a 重渐变 AI 仪表盘.
- **Don't** use 装饰性玻璃拟态, backdrop blur, gradient text, or ornamental bokeh/orb backgrounds.
- **Don't** add controls for real account linking, withdrawals, leverage contracts, or live broker trading.
- **Don't** use thick side-stripe borders as status decoration.
- **Don't** nest cards inside cards. Use unframed layouts or a single panel layer.
- **Don't** hide core functionality on mobile; adapt navigation and tables instead.
- **Don't** introduce English labels into strategy dialogs, settings, buttons, empty states, or alerts.
