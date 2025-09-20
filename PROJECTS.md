## 🌌 Ghost Stack Project Index
🔮 ghostllm

Enterprise-grade LLM proxy built in Rust.
OpenAI-compatible API with multi-provider adapters, smart routing, usage/cost tracking, and streaming. Acts as the central brain for all Ghost projects.
https://github.com/ghostkellz/ghostllm

⚡ zeke
Rust-native AI developer companion (CLI + TUI).
Brings Copilot, Claude, GPT, and local Ollama into your terminal. Refactors, explains, and batch-edits code through GhostLLM.
https://github.com/ghostkellz/zeke

📝 zeke.nvim
Neovim plugin replacing claude-code.nvim.
Floating panel, inline completions, diff previews, and /explain or /fix commands. Connects to GhostLLM (or OMEN) for AI support inside the editor.
https://github.com/ghostkellz/zeke.nvim

🛠 jarvis
CLI-native AI system & homelab copilot in Rust.
Manages Arch, Proxmox, Docker, DNS, VPNs, and crypto watchers. Integrates with GhostLLM/OMEN for reasoning while automating real commands and infra tasks.
https://github.com/ghostkellz/jarvis

🌐 ghostflow
Rust-native AI workflow engine (n8n alternative).
Visual drag-and-drop editor (Leptos), CLI (gflow), and execution engine. Wires together GhostLLM, Jarvis, and external APIs into automated workflows.
https://github.com/ghostkellz/ghostflow

🧠 omen
 (this project)
https://github.com/ghostkellz/omen
Open Model Exchange Network — the unified API layer.
Provides OpenAI-compatible + gRPC interfaces across all providers (Claude, GPT, Copilot, Grok, Gemini, Ollama, Bedrock, Azure). Designed as the canonical API spec for GhostStack, powering Zeke, Jarvis, GhostFlow, and GhostLLM.
