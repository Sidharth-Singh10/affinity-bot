![AffinityBanner](https://github.com/user-attachments/assets/e207e037-b436-4007-a0a6-8bc73e0a99dd)
# **Affinity Bot** 🤖
Affinity Bot is a powerful and customizable Discord bot designed for seamless interaction and automation within the Affinity Discord server. Built using modern tools and technologies, it leverages **Rust** for high performance and safety and **Shuttle** for effortless deployment.

---
## **Features** ✨
- 🔧 **Moderation Tools**: Manage your server with commands like kick, ban, mute, and more.
- 📚 **Utility Commands**: Fetch information, set reminders, and more!
- 🔗 **URL Shortener**: Create custom shortened URLs with the [CargoСut](https://github.com/Sidharth-Singh10/CargoCut) feature.
- 🎵 YouTube MP3 Downloader: Extract high-quality MP3 audio from YouTube videos with a simple command.
- ⚡ **High Performance**: Rust-based backend ensures speed, reliability, and memory safety.
- 🚀 **Serverless Deployment**: Leveraging Shuttle for efficient and scalable hosting.
---
## 🚀 High-Performance Architecture 🔄
The bot leverages Rust's async runtime **Tokio** for efficient parallel processing:
- 🔍 **Price Scraping Engine**: Runs independently in a dedicated async task, continuously monitoring and updating product prices without blocking the main bot operations.
- 📬 **Notification Manager**: Operates in parallel, checking and sending price alerts on configurable intervals. Uses async handlers for processing multiple notifications concurrently.
- ⚡ **Non-Blocking Design**: All database operations and external API calls are fully asynchronous, ensuring optimal resource utilization and responsiveness.
---
## **Tech Stack** 🛠️
- **Rust** 🦀: A memory-safe and fast programming language.
- **Shuttle** 🚀: A Rust-based serverless platform for easy deployment.
---
## **Getting Started** 🏁
Follow these steps to set up Affinity Bot locally or deploy it to production.
### **Prerequisites** 📋
- Install **Rust** 🦀: [Get Rust](https://www.rust-lang.org/tools/install)
- Install **cargo-shuttle** 🚀:
 ```bash
 cargo install cargo-shuttle
```
- Install sea-orm-cli
```bash
cargo install sea-orm-cli@1.1.0
```
- Run Migrations
```bash
sea-orm-cli migrate fresh
```
- Follow [Blog](https://blog.logrocket.com/building-rust-discord-bot-shuttle-serenity/) to setup your own bot
