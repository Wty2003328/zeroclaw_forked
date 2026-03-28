# Fork: Wty2003328/zeroclaw

This is a fork of [zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) with additional features focused on **WeCom integration** and **memory control**.

## Added Features

### WeCom Enterprise App Channel

Full WeCom (WeChat Work) Enterprise App support, enabling ZeroClaw to operate as a WeCom corporate application with secure encrypted messaging.

- **Dual-mode config**: enterprise app (`corp_id`/`corp_secret`/`agent_id`/`token`/`encoding_aes_key`) or legacy bot webhook (`webhook_key`)
- **AES-256-CBC message encryption/decryption** with PKCS7 padding (block size 32)
- **SHA1 signature verification** for callback authentication
- **XML message parsing** for WeCom's message format
- **OAuth access token management** with automatic refresh
- **Markdown stripping** for clean plaintext delivery in WeCom messages
- **Cron job delivery** via WeCom channel — schedule recurring agent tasks that report results directly to WeCom

### Auto Memory Recall

Configurable `auto_recall` setting that controls whether ZeroClaw automatically injects relevant memories into conversation context.

- New `auto_recall` field in `MemoryConfig` (defaults to `true` for backward compatibility)
- Integrated into the onboard wizard for first-time setup
- Allows users to disable automatic memory injection when not needed, reducing token usage