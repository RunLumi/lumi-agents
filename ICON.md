# ICON.md — Lumi Glyph System

Status: product icon guidance  
Source of truth: `DESIGN.md §9 Iconography`

## 1. Direction

Lumi uses a **Lumi Glyph System**, not a raw third-party icon pack.

Principle:

> Familiar for ordinary actions. Distinctive for Lumi concepts.

Use **Tabler Icons** as the donor library for generic utility actions, then expose icons through Lumi-owned components.

Do not make Lucide, Tabler, or any other third-party pack part of the product identity.

## 2. Three layers

### A. Utility glyphs

Use adapted Tabler icons for ordinary actions and objects:

- search
- plus / minus
- close
- chevrons
- menu
- copy
- download / upload
- refresh
- calendar / clock
- filter / sort
- file / folder
- settings
- terminal
- bell
- edit / trash
- external link

These should be visually quiet and familiar.

### B. Lumi product glyphs

Draw Lumi-native icons for concepts strategically important to the product:

- Project
- Agent
- Task
- Workflow
- Role
- Skill
- Memory
- Evidence
- Approval
- Artifact
- Verification
- Exception
- Guardrail
- Handoff
- Recovery
- Authority
- Provenance
- Work Mode
- Workflow Pack
- Role Pack

If a concept is distinctly Lumi, prefer a custom glyph.

### C. Semantic modifiers

Use consistent small modifiers instead of inventing new icons for every state:

- **clarity mark** → Lumi found / clarified / verified insight
- **annotation dot** → evidence / source / provenance
- **diagonal channel** → handoff / progress / active work
- **L-corner** → structure / project / bounded workspace

Object first, state second.

Example:

```text
Project
Project + annotation dot   = project with evidence
Project + clarity mark     = verified insight about project
Project + amber rail       = needs attention
Project + red marker       = blocked / failed
```

## 3. Geometry

Default construction:

```text
canvas          20 × 20
optical area    ~16 × 16
stroke          1.75px
small rendering 2px at 16px
angles          0° / 45° / 90° preferred
corners         geometric, restrained
fills           avoid except tiny semantic dots/markers
```

Icons should feel derived from the folded Lumi L-form: structured, precise, architectural.

Avoid soft blob geometry, excessive curves, or playful rounded shapes.

## 4. Color

Icons are monochrome by default.

```text
default      Civic Navy   #102A43
secondary    Soft Slate   #5E6677
selected     Lumi Blue    #006093
success      #1F7A4D
attention    #F4A62A
danger       #C2410C
```

Use status color only when it carries real meaning.

Do not decorate icons with extra color.

## 5. Usage rules

- Icons identify **objects or actions**.
- Typography carries **hierarchy**.
- Do not place an icon before every heading.
- Do not use robots to represent agents.
- Do not use brains to represent models.
- Do not use sparkles as a generic AI symbol.
- Use the clarity mark sparingly.
- Prefer one strong icon over multiple decorative symbols.
- Never mix icon packs directly in product code.

## 6. Implementation

Target API:

```tsx
import {
  ProjectGlyph,
  EvidenceGlyph,
  SearchGlyph,
  GitBranchGlyph,
} from "@lumi/glyphs";
```

Do not import Tabler directly across the app:

```tsx
// avoid
import { IconFolder } from "@tabler/icons-react";
```

Wrap/adapt donor icons inside `@lumi/glyphs` so geometry and implementation can evolve without refactoring product code.

Recommended structure:

```text
@lumi/glyphs
├── utility/   # normalized Tabler subset
├── product/   # Lumi-native semantic glyphs
├── marks/     # clarity, annotation-dot, L-corner, diagonal
└── index.ts
```

## 7. Agent decision rule

Before adding an icon:

1. Is this a generic action/object?
   - Reuse the closest normalized Tabler utility glyph.
2. Is this a Lumi-native product concept?
   - Use or create a Lumi product glyph.
3. Is this only a state?
   - Add a semantic modifier to the existing object glyph.
4. Does an icon materially improve recognition?
   - If not, use text only.

Never add a new glyph because “it looks nicer.”

## 8. Quality bar

A Lumi icon should pass all of these:

- understandable at 16–20px
- visually consistent beside Geist
- works in Civic Navy without fill
- still recognizable without color
- uses the same stroke and optical weight as the system
- does not look cartoonish, futuristic, magical, or generic-AI
- adds signal, not decoration

Long-term goal:

> A user should eventually recognize Project, Evidence, Agent, and Workflow glyphs as Lumi even without the word “Lumi” beside them.
