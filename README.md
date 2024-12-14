# PlugOvr

PlugOvr is a Rust-based application for AI Assistance. With one shortcut you can access PlugOvr from any application. PlugOvr is cross-platform and works on Linux, Windows and MacOS.

## Features

- Create your own prompts
- Choose for each template the LLM that performs best.
- Integrates Ollama Models 

## How to use

- Download PlugOvr from [PlugOvr.ai](https://plugovr.ai)
- Install PlugOvr
- select the text you want to use
- press Ctrl + Alt + I (Linux / Windows) or Ctrl + I (MacOS) write your own instructions.
- or use your favorite templates with Ctrl + Space
- select Replace, Extend or Ignore
- accept or reject the AI answer

## compile from source

### dependencies

Linux:
```bash
sudo apt install --no-install-recommends cmake build-essential libssl3 libdbus-1-3 libglfw3-dev libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxdo-dev



## build and run from source

```bash
cargo run --release
```
