# Agent Note: Scheduled upstream synchronization publishes desktop builds

Status: implemented

English | [中文](2026-08-21-automated-upstream-desktop-sync.zh.md)

## Problem

The QuantaBricks desktop distribution depends on DeepSeek Harness Web, so manually carrying upstream core changes delays Windows releases and can separate the shell from the service it opens.

## Decision

`.github/workflows/sync-upstream.yml` checks `deepseek-ai/deepseek-harness` `master` every day and on manual dispatch. When upstream has new commits, it merges them in the workflow checkout, installs dependencies, builds Harness Web, builds signed Windows Tauri installers, pushes the merge to `main`, and publishes a uniquely versioned GitHub Release. Manual dispatch can also publish the current `main` to bootstrap the updater.

The workflow pushes only after both builds succeed. A merge conflict, dependency failure, or installer failure leaves `main` and the latest release unchanged. The Tauri updater checks the latest release at application startup and accepts only artifacts signed by the private key stored in GitHub Actions.

## Alternatives considered

**Manual synchronization** leaves desktop releases dependent on an operator noticing every upstream change and rerunning the full build sequence.

**Pushing the merge before validation** can publish a core revision without a working Windows installer.

**Opening a synchronization pull request** preserves review but does not provide the automatic core update and release requested for this distribution.

## Consequences

Harness core changes that merge cleanly produce a Windows release without a local build. Installed shells download the next signed release at startup. Upstream conflicts still require a maintainer to resolve them, and scheduled builds consume GitHub Actions capacity.
