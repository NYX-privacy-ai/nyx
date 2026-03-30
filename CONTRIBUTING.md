# Contributing to Nyx

Thanks for your interest in contributing to Nyx! This document covers the development setup and guidelines.

## Development Setup

### Prerequisites

- **macOS 12+** (Monterey or later)
- **Node.js 22+**
- **Rust 1.80+** (via [rustup](https://rustup.rs))
- **Docker Desktop** (for the agent container)

### Getting Started

```bash
# Clone the repo
git clone https://github.com/NYX-privacy-ai/nyx.git
cd nyx

# Install frontend dependencies
npm install

# Start the dev server with hot reload
npm run tauri dev
```

This launches the SvelteKit dev server on `localhost:1420` and the Tauri native window with hot module replacement.

### Project Structure

```
src/                    → SvelteKit frontend (Svelte 5, Tailwind CSS)
src-tauri/src/          → Rust backend (Tauri v2 commands)
src-tauri/resources/    → Bundled files copied to ~/openclaw/ on setup
src-tauri/capabilities/ → Tauri security permissions
```

### Building

```bash
# Frontend only
npm run build

# Full Tauri build (produces .dmg + .app)
npm run tauri build
```

### Code Style

- **Frontend:** Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`). No shared stores — all state is component-local.
- **CSS:** Tailwind utility classes. Custom design tokens are defined in `tailwind.config.js`.
- **Rust:** Standard `rustfmt` formatting. All Tauri commands are `async` and return `Result<T, String>`.
- **Naming:** Components use PascalCase. Routes use SvelteKit conventions (`+page.svelte`, `+layout.svelte`).

### Design Tokens

The app uses a custom dark theme defined in `tailwind.config.js`:

| Token | Value | Usage |
|---|---|---|
| `black` | `#18181B` | Background |
| `surface` | `#1F1F23` | Card backgrounds |
| `surface-raised` | `#27272B` | Elevated surfaces |
| `ivory` | `#FAFAFA` | Primary text |
| `ivory-muted` | `#A1A1AA` | Secondary text |
| `accent` | `#6366F1` | Interactive elements |
| `positive` | `#4ADE80` | Success states |
| `warning` | `#FBBF24` | Warning states |
| `negative` | `#F87171` | Error states |
| `border` | `#303036` | Borders, dividers |

### Adding a Tauri Command

1. Write the function in the appropriate Rust module (e.g., `src-tauri/src/config.rs`)
2. Annotate with `#[tauri::command]`
3. Register in `main.rs` `invoke_handler`
4. Call from the frontend: `const result = await invoke('command_name', { args })`

### Pull Requests

- Keep PRs focused on a single change
- Ensure `npm run build` and `cargo build` pass
- Test the change in the Tauri dev environment (`npm run tauri dev`)
- Write clear commit messages describing the "why"

## Reporting Issues

Open an issue on GitHub with:

- macOS version and architecture (Apple Silicon / Intel)
- Steps to reproduce
- Expected vs actual behaviour
- Console output if applicable
