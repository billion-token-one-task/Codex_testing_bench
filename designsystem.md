# TokenMart-Inspired Design System

## Purpose

This document captures the UX and visual design system used by TokenMart and turns it into a clear guideline for any webapp that wants to follow the same spirit.

Scope:

- marketing landing pages
- product application shells
- documentation sites
- auth, connect, or onboarding checkpoints

This is not a generic modern SaaS system.

It is a design language for products that should feel:

- editorial instead of app-store polished
- operational instead of decorative
- instrumented instead of casual
- high-signal instead of comfort-first
- branded through structure and typography, not through soft gradients and rounded cards

Use this document as a practical rulebook. When a choice is unclear, prefer the more legible, more structured, more mission-control-like option.

---

## Core Style In One Sentence

Build the interface as if a serious editorial publication, a mission-control console, and a technical operator dashboard were merged into one hard-edged web system.

---

## What This System Is

This style is a blend of a few very specific families:

- Swiss and neo-grotesque editorial typography
- newspaper and magazine layout logic
- industrial signage and specimen-label graphics
- terminal and telemetry readouts
- CRT, scanline, barcode, and print-texture nostalgia
- infrastructure-console navigation patterns
- cartography and network-diagram metaphors

This style is not:

- glassmorphism
- soft consumer fintech
- generic developer dashboard UI
- cyberpunk neon-on-black overload
- playful retro pixel art
- luxury minimalism

The key distinction is this:

The system uses retro-tech motifs, but it treats them as information architecture and interface chrome, not as cosplay.

---

## Design Thesis

The interface should feel like a live operating surface for consequential work.

Users should feel that:

- the product has a memory
- the system has rules
- actions have operational consequences
- navigation is structured, not casual
- information has hierarchy and pressure
- the interface can show both overview and evidence

Every major surface should communicate at least one of these:

- status
- route
- pressure
- priority
- authority
- system context

If a screen feels merely "pretty," it is off-style.

Another useful test:

If the same screen could belong to a generic fintech, a productivity startup, or an AI wrapper, it is too generic for this system.

---

## Typography

## Typographic Strategy

This system runs on a strict three-family model:

- body sans for readable narrative copy
- condensed display sans for titles and high-impact labels
- mono for telemetry, metadata, system labels, counts, and controls

In the current codebase, the effective stack is mostly:

- body: `Avenir Next`, `Segoe UI`, fallback sans
- display: `Arial Narrow`, `Helvetica Neue Condensed`, fallback sans
- mono: `SFMono-Regular`, `Menlo`, `Monaco`, fallback monospace

There are package dependencies for more curated fonts, but the live style mostly depends on these system-family behaviors. That matters because the visual identity is coming from classification and usage, not from a boutique font file.

## Recommended Font Choices For New Projects

If you want to reproduce this spirit in another webapp, use this selection logic:

### Best-case stack

- body: Avenir Next, Neue Haas Grotesk Text, or a comparable clean humanist sans
- display: Helvetica Neue Condensed, Arial Narrow, Akzidenz-Grotesk Condensed, or a similarly severe condensed sans
- mono: SF Mono, IBM Plex Mono, JetBrains Mono, or Menlo

### Safer web-friendly substitutes

- body: Geist Sans, IBM Plex Sans, or Source Sans 3
- display: Barlow Condensed, IBM Plex Sans Condensed, Roboto Condensed, or Archivo Narrow
- mono: IBM Plex Mono or JetBrains Mono

### Important rule

Do not choose a quirky display font to "make it unique."

This style depends on disciplined, institutional typography. The display face should feel like it belongs on a report cover, a transit sign, or a magazine spread, not on a poster for a music festival.

## Font Roles

### 1. Body Sans

Use a clean, neutral, modern sans with humanist warmth.

Recommended character:

- clear at small sizes
- not too geometric
- not too quirky
- slightly refined, not utilitarian

Use it for:

- paragraphs
- explanatory copy
- helper text
- long-form interface descriptions
- UI text that needs calm readability

Avoid using the body family for:

- page titles
- telemetry strips
- micro-labels

Target feel:

- competent
- neutral
- clean
- contemporary

### 2. Display Sans

Use a condensed sans for nearly all page and section titles.

This is one of the most important choices in the system.

The display family should feel:

- condensed
- uppercase-friendly
- editorial
- assertive
- slightly severe

Use it for:

- hero headlines
- page titles
- section titles
- stat values when emphasis is needed
- route cards
- major navigation labels

The display type should create the main identity. In TokenMart, the feeling comes from condensed all-caps headlines with tight leading and large scale.

The current system also defines several "pixel" variants, but they are really usage aliases of the same condensed display family. That is the right idea. If you want the same spirit, do not switch into literal bitmap fonts. Keep the forms crisp, condensed, and typographic.

Good references in spirit:

- classic magazine cover typography
- newspaper section headers
- industrial wayfinding
- institutional report covers

Avoid:

- rounded display fonts
- friendly startup grotesks
- tech-bro geometric sans defaults
- serif display fonts for primary app headings

### 3. Monospace

Use mono aggressively, but only for the right jobs.

Use it for:

- telemetry chips
- labels
- breadcrumb codes
- status indicators
- badges
- metadata
- table numerics
- button text when the button is operational
- path strings
- system identifiers
- helper rails

The mono usage is what makes the interface feel instrumented.

Mono text should usually be:

- uppercase
- tracked out
- compact
- small
- used as a frame around more expressive display typography

Avoid setting entire long paragraphs in mono unless the content is truly console-like.

## Typographic Rules

### Headings

- almost always uppercase
- use condensed display family
- use tight leading between `0.82` and `0.95`
- use slightly tight tracking, often negative or near-zero
- let headings be large and dominant

### Labels

- mono
- uppercase
- letter-spaced
- small
- often muted in gray or brown-gray
- frequently paired with a line, barcode, dot, or divider

### Paragraphs

- body sans
- moderate line height
- restrained width
- slightly warm gray rather than pure black

### Numbers

- use tabular numerals in mono or display contexts where comparison matters
- make large values bold and immediate
- support them with small labels above or below

## Typographic Do / Don't

Do:

- pair condensed display with mono labels
- create strong scale contrast
- let the title dominate the card
- use uppercase as a structural device

Don't:

- use four or five unrelated families
- use soft UI fonts like default Inter everywhere
- use display type for body copy
- use mono for everything

---

## Inspiration Families

## 1. Editorial Modernism

This system borrows heavily from editorial design:

- strong headline hierarchy
- asymmetrical emphasis
- large type blocks
- framed sections
- clear reading lanes
- bold issue-cover energy on landing pages

Use this influence when designing:

- hero sections
- page openers
- content decks
- documentation indexes

What to copy:

- bold title blocks
- high contrast between heading and body
- explicit reading lanes
- page sections that feel named and published

What not to copy:

- delicate luxury whitespace
- serif-heavy magazine aesthetics
- poster-like chaos with weak hierarchy

## 2. Mission Control

The app shell behaves like mission control:

- fixed left rail
- telemetry banner
- path strip
- coded section identity
- visible status
- explicit active surface

Use this influence when designing:

- shell layout
- control surfaces
- admin pages
- dashboards
- work queues

What to copy:

- layered shell structure
- visible route context
- explicit active state
- persistent operational information

What not to copy:

- military fantasy styling
- excessive warning colors
- dark-room hacker cliches

## 3. Industrial Labeling

A lot of the flavor comes from specimen and labeling systems:

- barcode marks
- code stamps
- bracket corners
- strip headers
- ID-like labels
- status markers

Use this influence when designing:

- badges
- card headers
- metadata rows
- system chips
- filter controls

What to copy:

- code stamps
- barcode strips
- specimen-like framing
- status markers that feel printed or encoded

What not to copy:

- fake serial-number clutter everywhere
- decoration with no informational purpose

## 4. Retro Display Technology

CRT and print artifacts are present, but controlled:

- scanlines
- hatch grids
- dither
- halftone
- phosphor-like overlays
- glitch accents

Use this influence as atmosphere, not as the core content layer.

The effect should suggest active signal processing, not gaming nostalgia.

Good usage:

- one scanline overlay in a shell
- subtle glitch on a hero word
- controlled marquee for live status

Bad usage:

- full-page animated distortion
- chromatic aberration on normal text
- constant glitch on core interface controls

## 5. Cartography And Network Diagrams

The product language also feels mapped and routed:

- mountains
- routes
- channels
- threads
- feeds
- lattice
- portals
- towers

This means ornament should often imply:

- flow
- topology
- linked systems
- distributed work

Not all decoration should be abstract blobs. It should feel diagrammatic.

That means:

- lines, lattices, nodes, rails, routes, and frames are good
- amorphous blobs and soft gradient clouds are usually wrong

---

## Color System

## Base Palette

The palette is mostly built from:

- off-white canvas
- white and near-white surfaces
- charcoal and near-black text
- hot magenta as the primary signal color
- pale blush backgrounds for accent surfaces

Representative colors from the current system:

- canvas: `#fafafa`
- strong surface: `#ffffff`
- primary text: `#0a0a0a`
- secondary text: `#404040`
- tertiary text: `#737373`
- primary brand: `#e5005a`
- deep brand: `#b80048`
- secondary brand: `#ff6b9d`
- soft brand wash: `#fff0f5`

## How Color Is Used

### Black / Ink

Black is structural.

Use it for:

- borders
- text
- table headers
- shells
- dividers
- high-authority surfaces

### Magenta / Pink

Pink is not a broad fill color.

It is a signal color.

Use it for:

- active states
- accents
- top stripes
- focus surfaces
- key CTA fills
- viewfinder brackets
- progress emphasis
- important count highlights

### Soft Pink Wash

Use blush pink surfaces when you need:

- emphasis without full saturation
- hover states
- cards with lifted priority
- selected states that should remain readable

### Semantic Colors

The system includes semantic success, warning, and error tones, but they are secondary to the black-and-magenta language.

Use semantic tones carefully:

- success for trust or healthy status
- warning for pressure or degraded state
- error for actual fault or blocking state

Do not let semantic colors overwhelm the main design identity.

## Color Rules

Do:

- keep the canvas light
- let pink behave like a controlled alert light
- rely on black borders and ink for structure
- use muted neutrals for most supporting text

Don't:

- flood entire pages with brand pink
- use rainbow accents
- use soft gray everything with one pink button
- default to dark mode unless the product really needs it

---

## Layout System

## Overall Structure

This design language works best with explicit shell layers.

A typical app page should have:

1. global navigation shell
2. route context
3. section header
4. summary or telemetry layer
5. main working canvas

In practice, that often means:

- left sidebar
- breadcrumb row
- focus banner or pinned status rail
- thin data strip
- framed content card

## Widths

Use a broad but controlled canvas.

Recommended pattern:

- full-bleed background atmosphere
- content constrained to `1280px` to `1480px`
- large shells should feel expansive
- reading surfaces should be narrower

Docs and reference pages should use:

- a left navigation rail
- a main reading column
- an optional right anchor rail

Auth and access surfaces should use:

- a centered main card
- an optional explanatory side dossier on desktop

## Grid Behavior

Prefer:

- strong two-column grids
- stat blocks in 2x2 or 4-up arrangements
- cards with hard separators
- zero-gap or minimal-gap grids when the layout should feel systemic

Avoid:

- overly soft card masonry
- floating disconnected components
- excessive whitespace that weakens urgency

## Section Framing

Sections should usually be framed with at least one of:

- a hard border
- a top or side accent strip
- a viewfinder bracket
- a barcode or telemetry label
- a distinct internal divider

This system likes surfaces that feel contained and named.

---

## Surface Treatment

## Card Philosophy

Cards should feel like panels, dossiers, or instrument modules.

Core card traits:

- square corners
- visible border
- white or near-white background
- minimal blur
- strong hover state
- optional texture overlay

Use these card variants:

- default panel
- highlighted mission panel
- glass panel with hard border
- specimen card with top bar and ID stamp
- gradient-border emphasis card

## Texture Use

Texture is part of the style, but it must stay behind content.

Allowed texture families:

- grain
- halftone
- crosshatch
- dither
- scanline
- hatch grid
- barcode strip
- newspaper-like line noise
- engraving-style diagonal rules

Texture rules:

- keep opacity low
- use masking so texture lives at edges or corners
- never let texture reduce readability
- treat texture as atmosphere and framing

Best usage:

- backgrounds
- hero sections
- shell surfaces
- accent panels
- empty states

Worst usage:

- behind dense body copy
- under small inputs
- inside high-frequency tables

---

## Navigation Guidelines

## Sidebar

The primary shell should feel like an operator rail.

Sidebar rules:

- group items by domain, not alphabetically
- each domain gets a label
- active route is obvious
- code-like route markers are welcome
- include one high-priority pinned action or live pin
- mobile version should preserve the same structure, not become a totally different nav pattern

## Breadcrumbs And Pathing

The system likes explicit route context.

Use:

- breadcrumbs
- path strips
- section labels
- context codes

Users should not wonder where they are inside the product.

## Command Palette

A command palette fits this design language very well.

It should:

- look like a framed console object
- use mono labels
- group by section
- show path or route metadata
- support keyboard-first flow

---

## Headers And Hero Blocks

## Page Headers

A TokenMart-style page header should include:

- small eyebrow label
- large condensed title
- optional short description
- section identity
- optional actions on the right
- telemetry or chip strip underneath

Good page headers feel:

- named
- official
- directional

Not:

- breezy
- app-store friendly
- conversational

## Landing Heroes

The landing page is the most theatrical surface in the system.

Use it for:

- bold headlines
- dramatic scale
- world-building
- route choice
- controlled motion

Landing hero rules:

- very large display type
- strong asymmetric composition
- one vivid accent color
- specimen-style support panel
- navigation CTAs with different authority levels

Do not make the rest of the product this dramatic. The landing page is allowed to be louder than the app shell.

---

## Components

## Buttons

Buttons should feel operational.

Use:

- square corners
- mono uppercase labels
- compact padding
- strong border change on hover
- pink primary fill
- black secondary fill or outlined secondary states

Primary CTA:

- pink background
- white text
- black or deeper-pink hover

Secondary CTA:

- white background
- black border
- pink hover or black hover

Ghost CTA:

- minimal fill
- only for tertiary actions

Avoid pill buttons, soft shadows, or soft pastel hover states.

## Tabs And Filters

Tabs should feel like mode switches, not soft segmented controls.

Use:

- hard bottom borders or full box borders
- mono uppercase labels
- optional count chips
- clear active inversion

Filters should usually be:

- chip-like
- compact
- bordered
- dense

## Stats

Stats are central to this design system.

Each stat should contain:

- small mono label
- large value
- optional delta
- optional bracket or accent
- visible hover state if interactive

Stats should feel alive, not decorative.

## Tables

Tables should feel like ledgers or operator logs.

Use:

- dark or strongly framed headers
- mono numerics
- clear row separators
- subtle hover accent
- structured density

Avoid soft data grids with faint invisible rules.

## Modals

Modals should look like interrupt surfaces, not floating glass clouds.

Use:

- hard border
- top accent stripe
- darkened backdrop
- optional scanline or framed edges

The modal should feel like a formal system intervention.

## Notices

Inline notices should use:

- left accent bar
- hard border
- clear tone
- mono label
- concise language

They should feel like an annotated system state.

---

## Motion

## Motion Philosophy

Motion should imply system activity, signal flow, and live state.

It should not imply playfulness.

Use motion in three categories:

### 1. Entrance Motion

Use short reveal animations for:

- hero elements
- page headers
- counts
- cards entering the viewport

Timing:

- around `0.4s` to `0.6s`
- eased, crisp, not bouncy

### 2. Ambient Motion

Use sparingly for:

- marquees
- scanline sweeps
- floating particles
- subtle glitch overlays
- slow rotations

Ambient motion should live in the background.

### 3. Interaction Motion

Use for:

- hover color changes
- slight translate shifts
- border emphasis
- focus states

Timing:

- around `120ms` to `200ms`

## Motion Rules

Do:

- animate transform and opacity first
- support reduced motion
- keep movement crisp and intentional

Don't:

- use springy playful motion
- animate everything on every page
- use parallax-heavy marketing gimmicks
- make reading surfaces unstable

---

## Copy And Voice

This system is part graphic and part editorial. Copy matters.

Preferred voice:

- direct
- declarative
- specific
- operational
- slightly institutional

Good patterns:

- "Mountain Feed"
- "Mission Control"
- "Ranking Policy"
- "Current Mode"
- "Route Directory"
- "Publish Signal"

Bad patterns:

- "Let’s get started!"
- "Welcome back, friend"
- "Supercharge your workflow"
- "Magic AI tools for everyone"

The system should sound like it understands coordination, infrastructure, and protocol, not like a growth landing page.

---

## Design Rules By Surface Type

## Marketing Surface

Allowed:

- largest headings
- strongest motion
- most expressive composition
- most overt brand theater

Required:

- one dominant message
- clear route choices
- specimen or system panel support

## Product Shell

Allowed:

- dense rails
- telemetry
- dashboards
- framed working canvases

Required:

- explicit section identity
- strong navigation
- path awareness
- operational status language

## Documentation Surface

Allowed:

- calmer pacing
- more whitespace
- longer text
- directory structures

Required:

- still use the same typography and framing DNA
- still feel branded
- prioritize readability over theater

## Auth Or Access Surface

Allowed:

- central card
- step rails
- checklists
- spec grids

Required:

- feel like an entry checkpoint
- feel formal and consequential
- stay simpler than the main app shell

---

## Accessibility And Usability Guardrails

Follow the spirit of the system, but improve where needed.

Keep:

- strong visual hierarchy
- visible framing
- structured navigation
- keyboard-friendly control patterns

Improve:

- use visible focus styles everywhere
- do not rely only on pink to communicate state
- avoid `transition: all` when implementing new controls
- pair every input with a real label
- keep headings and body copy readable at normal zoom
- ensure decorative textures never interfere with contrast

When in conflict, readability wins over aesthetic texture.

---

## Concrete Build Checklist

If you are designing a new webapp in this style, confirm all of these:

- The app uses one body sans, one condensed display family, and one mono family.
- The main shell is structured into named layers.
- Every major screen has a clear title, label, and route context.
- Borders are strong and corners are square.
- Pink behaves like a signal color, not a wallpaper color.
- There is at least one telemetry or metadata layer on important screens.
- Cards feel like panels, not generic containers.
- Textures are present but restrained.
- The landing page is more theatrical than the app shell.
- Docs are calmer but still visibly from the same system.
- Buttons and tabs feel operational.
- The interface feels deliberate and slightly severe, not soft and friendly.

If fewer than eight of these are true, the design is probably drifting away from the spirit of the original.

---

## Practical Starter Tokens

Use this as a starting point for implementation:

```css
:root {
  --canvas: #fafafa;
  --surface-0: #ffffff;
  --surface-1: #f5f5f5;
  --text-primary: #0a0a0a;
  --text-secondary: #404040;
  --text-tertiary: #737373;
  --border-default: rgba(10, 10, 10, 0.12);
  --border-strong: rgba(10, 10, 10, 0.22);
  --brand: #e5005a;
  --brand-deep: #b80048;
  --brand-soft: #fff0f5;
  --success: #059669;
  --warning: #d97706;
  --error: #dc2626;
  --font-body: "Avenir Next", "Segoe UI", sans-serif;
  --font-display: "Arial Narrow", "Helvetica Neue Condensed", sans-serif;
  --font-mono: "SFMono-Regular", "Menlo", "Monaco", monospace;
}
```

Recommended defaults:

- app background: off-white
- container borders: 2px black or near-black
- title family: display
- label family: mono
- corner radius: 0
- primary accent: magenta

---

## Final Rule

If you want another webapp to feel like this one, do not start from "dashboard components."

Start from:

- a typographic hierarchy
- a shell structure
- a label system
- a border system
- a restrained accent-color logic
- an operational worldview

The style comes from disciplined composition, not from adding pink after the fact.
