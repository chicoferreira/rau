---
name: featured-project
description: Scaffold a new built-in example / featured project for the rau renderer. Use this whenever the user wants to add, create, or generate a featured project, a bundled example scene, a new GenerateTemplate, or a new entry in the main-menu "Featured Projects" gallery. Covers writing the src/scene/<name>.rs scene builder, registering the GenerateTemplate, adding the WGSL shaders and any model/texture assets, and registering the FeaturedProject card. Trigger even if the user just describes the scene they want ("add a featured project that renders a spinning cube") without naming the mechanism.
---

# Featured project scaffolding

A featured project in rau is a bundled example that (1) builds its `project.json`
programmatically from Rust code in `src/scene/`, (2) ships its shaders/assets in
`projects/<name>/`, and (3) shows up as a card in the main-menu gallery. The
recent ones — `model`, `game-of-life`, `full-example` — all follow this pattern.

This skill scaffolds the **project resources**: the scene builder code, the
generator registration, the shaders/assets, and the gallery card. Two steps are
done by the user manually afterward — see "Hand off to the user" at the end. Do
not run them yourself.

## What you produce

Pick a kebab-case `<name>` (the project folder + featured id, e.g.
`game-of-life`) and a `PascalCase` `<Variant>` for the generate template (e.g.
`GameOfLife`). Then make these changes:

### 1. The scene builder — `src/scene/<name>.rs`

Write `pub async fn create_scene(device, size, file_storage) -> AppResult<Project>`.
This is where the real work is: it registers shaders, textures, bind groups,
pipelines, passes, a viewport, etc. onto a `Project` and returns it.

**Read `references/resource-api.md` before writing this** — it's the cookbook for
the resource API (constructors, the `register` pattern, the WGSL binding
convention, model loading, compute ping-pong, cameras, uniforms). Then open the
closest existing scene in `src/scene/` (`game_of_life.rs` for compute/2D,
`model.rs` for a single 3D model, `full_example.rs` for the works) and adapt it
rather than writing from a blank file — matching the surrounding style and the
exact API shapes matters more than inventing your own.

### 2. Register the template — `src/scene/mod.rs`

Three edits, mirroring the existing variants:

```rust
pub mod <name>;                       // module declaration near the top

pub enum GenerateTemplate {
    // ...
    <Variant>,                        // new variant
}

// inside generate_project_async's `match template { ... }`:
GenerateTemplate::<Variant> => <name>::create_scene(&device, size, &file_storage).await?,
```

`GenerateTemplate` derives `clap::ValueEnum`, so the CLI name is the kebab-case
form of the variant (`GameOfLife` → `game-of-life`). Keep `<name>` and the
kebab-cased `<Variant>` identical so the folder, the CLI arg, and the featured id
all line up.

### 3. The shaders and assets — `projects/<name>/`

Create the folder and write every `.wgsl` file the scene references by
`FilePath`, plus any model/texture assets (`.obj`, `.mtl`, textures, `.hdr`).

The shaders' `@group`/`@binding` indices **must** match how the scene wires its
bind groups — this is positional and easy to get wrong. The binding convention is
spelled out in `references/resource-api.md`; cross-check every shader against the
bind-group/pipeline wiring before finishing.

If the scene loads an OBJ model, the model files must exist here **before** the
generate command runs, because `create_scene` loads the OBJ to build the material
bind groups. (`thumbnail.png` is added by the user later — don't fabricate it.)

### 4. The gallery card — `src/ui/components/main_menu/featured_projects.rs`

Append a `FeaturedProject` to the `FEATURED_PROJECTS` array:

```rust
FeaturedProject {
    id: "<name>",
    name: "Human Readable Name",
    owner: "chicoferreira",
    repo: "rau",
    git_ref: "main",
    path: "projects/<name>",
    description: "One or two sentences: what it renders and which rau features it shows off.",
},
```

Match the tone of the existing descriptions — concrete about the technique, a bit
inviting. The card loads its thumbnail from `projects/<name>/thumbnail.png` in
the repo, so the user must commit that file for the image to appear.

## Verify

Build to catch wiring mistakes before handing off:

```bash
cargo check
```

Fix any errors. A clean `cargo check` means the scene code and registration are
consistent; it does not validate shader bindings at runtime (that needs the
generate + run steps the user does).

## Hand off to the user

These two steps are intentionally manual — tell the user exactly how, but do not
do them yourself:

1. **Generate `project.json`:**

   ```bash
   cargo run -- generate <name> projects/<name>
   ```

   (`<name>` here is the kebab-case template name, e.g. `game-of-life`.) This
   writes `projects/<name>/project.json` from the scene builder.

2. **Capture the thumbnail:** open the project, then use the Texture's **"Save as
   Image"** feature to export the viewport at **1920x1080**, and save it as
   `projects/<name>/thumbnail.png`.

Finish by summarizing the files you created/changed and listing these two
commands/actions for the user to run.
</content>
