# Botanical field guide visual thesis

## Direction and rationale

Client Decision Receipt is designed as a **working botanist's field guide**: proposals are specimens, their chosen scope is pressed into the record, and each decision receives a stable catalogue mark. This gives an administrative action a calm, trustworthy physical metaphor without pretending to be a legal-signature product. It is deliberately not a startup dashboard, document suite, or generic gradient landing page.

The experience uses a paper field-note canvas, fine ink rules, specimen labels, and one original cyanotype-style illustration. Decoration explains the model: an unresolved proposal is a living sprig; a receipt is the same specimen flattened, labelled, and preserved.

## Tokens

- **Canvas — herbarium paper:** `#F3EFE2`; dark treatment `#17211C`
- **Surface — specimen sheet:** `#FFFCF2`; dark `#202C25`
- **Ink — carbon:** `#17251E`; dark `#F2EEDF`
- **Muted ink:** `#58655D`; dark `#B5C0B8`
- **Primary — bottle green:** `#1F5B44`; contrast `#FFFFFF`
- **Secondary — cyanotype blue:** `#295C73`
- **Success — fern:** `#23623F`; **warning — ochre:** `#8A5915`; **danger — madder:** `#9A3E35`
- Fine rules use `#C8C2AE` (light) / `#4A5A50` (dark). Text and controls meet 4.5:1; focus is a 3 px ochre/cream double ring and never relies on colour alone.

Light is the primary treatment because a decision receipt should read like paper. Dark mode is a fully painted nocturnal field-desk treatment selected through `prefers-color-scheme`, not an unthemed inversion.

## Type and spacing

- **Display:** Georgia, `Iowan Old Style`, `Palatino Linotype`, serif. Familiar printed-reference authority with no network or font payload.
- **Working text:** `Avenir Next`, Avenir, `Segoe UI`, system sans-serif. Neutral, direct labels and form copy.
- Scale: 16 px body / 18 px lead / 22 px h3 / 30 px h2 / clamp(40–64 px) h1. Long text maxes at 68 characters; receipt metadata uses tabular figures.
- An 8 px base rhythm: 4, 8, 12, 16, 24, 32, 48, 64, 96. Layout uses generous unboxed grouping; bordered cards only represent independent proposal records or literal receipt sheets.

## Interaction grammar

- Buttons are compact inked labels with square-ish 6 px corners, like accession stamps; touch targets remain at least 44 px.
- Accept, request changes, and decline are three explicit labelled controls with a persistent selected state and explanatory copy. No ambiguous icon-only actions.
- Submitted decisions become a read-only receipt sheet with a catalogue number, UTC timestamp, exact frozen line items, and a visible SHA-256 seal.
- New content enters as a sheet sliding a few pixels up from its physical origin. Async controls immediately show working text and an `aria-live` result. Destructive deletion names what it removes and requires confirmation.
- On phone, ornamental margin notes disappear; form groups and decision controls stack; the primary action stays in document order (no fixed bar over safe areas).

## Motion policy

UI transitions are 180–240 ms and animate opacity/transform only: a pressed stamp moves 1 px, notices fade, and receipt sheets settle upward 8 px. There are no loops or parallax. With `prefers-reduced-motion: reduce`, all movement and smooth scrolling become instant while hierarchy, state borders, and labels remain complete.

## Original asset plan and provenance

- `assets/src/herbarium-receipt.png`: generated specifically for this product on 2026-08-28 using the factory Azure image deployment (`factory-image`). Original generated imagery; no third-party asset or brand is represented.
- Runtime derivatives: `frontend/public/herbarium-receipt-640.webp` and `herbarium-receipt-960.webp`, cropped only and encoded locally. The 640 px hero is the mobile source and must remain under 300 KB.
- `frontend/public/og-receipt.jpg` is a local 1200×630 crop of the same original illustration for social previews; it contains no text and is 100 KB.
- Hand-authored UI marks (leaf sprig, check, change, decline, seal) are inline SVG with `currentColor`; no icon library.

### Prompt sheet

Use case: `illustration-story`  
Asset type: wide landing-page field-guide hero  
Primary request: an editorial botanical plate that visually connects an undecided proposal to a preserved decision receipt  
Scene/backdrop: warm blank herbarium paper, a single pressed maidenhair fern specimen laid diagonally across a slender archival receipt card, a small blank catalogue tag tied with cotton thread, faint cyanotype leaf shadows  
Style/medium: meticulous 19th-century botanical field-guide plate interpreted with contemporary risograph ink texture; tactile, calm, credible, not photorealistic  
Composition/framing: landscape, isolated still life, generous quiet negative space around the specimen, no frame, no interface mockup  
Lighting/mood: soft north-window light, contemplative and conclusive  
Color palette: bottle green, carbon ink, cyanotype blue, ochre thread, warm cream paper  
Materials/textures: pressed fern, cotton rag paper, deckled edges, subtle ink grain  
Constraints: one botanically plausible fern; blank tag; original illustration; no people  
Avoid: text, letters, numerals, signatures, logos, watermark, gradients, glossy 3D, corporate stock imagery, medical imagery, excessive props

Generated imagery is disclosed in the site footer.
