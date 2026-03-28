# Fork: Wty2003328/zeroclaw

This is a fork of [zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw) with additional features focused on **WeCom integration**, **agent safety**, and **task planning**.

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

### Channel-Based Tool Approval

Interactive approval workflow for dangerous commands via messaging channels. Previously, tools needing approval were silently denied in non-CLI mode. Now users can approve or deny tool calls directly from WeCom, Telegram, or any other channel.

- **Opt-in**: `enable_channel_approval = true` in `[autonomy]` config
- **Async approval flow**: agent sends approval request to channel, waits for user reply (`approve` / `deny` / `always`)
- **Configurable timeout**: `channel_approval_timeout_secs` (default 5 minutes, auto-deny on timeout)
- **Scoped by sender/thread**: concurrent approvals from different users/threads don't conflict
- **Audit trail**: all approval decisions (including timeouts) logged with channel and timestamp

### Multi-Step Task Planner

Two new tools (`plan` and `plan_update`) that let the agent declare and track a structured plan before executing complex tasks.

- Agent calls `plan` to declare steps upfront, visible to the user as a checklist
- Agent calls `plan_update` to mark steps as in-progress/completed/failed
- Plain-text checklist format works across all channels
- Step status indicators: pending, in-progress, completed, skipped, failed