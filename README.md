# betting-tools

**Use it on your phone or any browser:**  
https://jriordan55.github.io/betting-tools/

That is the Rust math engine in the browser (same `bettor-core` as the desktop
app — not Streamlit). Your book is already loaded. Add bets one at a time; they
stay on that device.

**Windows desktop installer:**  
https://github.com/jriordan55/betting-tools/releases/latest

---

Sports betting calculators and simulations. A Tauri v2 desktop app with a Rust
math engine — 26 calculators, a bet log, and a bundled reference library, all
offline.

![Bettor Desktop](site/assets/screenshot.png)

> [!IMPORTANT]
> **Early development.** This repository is being built in public. Interfaces,
> behavior, and the database schema may change without notice.

## Follow or run the project

Open the Pages link above on any phone or computer. For the full desktop shell
on Windows, download the latest release, or build from source:

```bash
git clone https://github.com/jriordan55/betting-tools.git
cd betting-tools
pnpm install
pnpm tauri build
```
