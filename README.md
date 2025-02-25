# PlugOvr

[![CI Status](https://github.com/PlugOvr-ai/PlugOvr/actions/workflows/check_everything.yml/badge.svg)](https://github.com/PlugOvr-ai/PlugOvr/actions)

PlugOvr is a Rust-based application for AI Assistance that integrates with your favorite applications. With one shortcut you can access PlugOvr from any application. PlugOvr is cross-platform and works on Linux, Windows and MacOS.

Select the text you want to use, write your own instructions or use your favorite templates.

![shortcuts](https://plugovr.ai/images/shortcuts.jpg)

## Features

- Create your own prompts
- Choose for each template the LLM that performs best.
- Integrates Ollama Models 

## How to use

- Download PlugOvr from [PlugOvr.ai](https://plugovr.ai)
- Install PlugOvr
- select the text you want to use
- press Ctrl + Alt + I  or Ctrl + I  write your own instructions.
- - Ctrl + I is enough but might conflict with shortcuts from other application e.g. making text italic in gmail.

- or use your favorite templates with Ctrl + Space
- select Replace, Extend or Ignore
- accept or reject the AI answer

## compile from source

### dependencies

Linux:
```bash
sudo apt install --no-install-recommends cmake build-essential libssl3 libdbus-1-3 libglfw3-dev libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxdo-dev
```


## build and run from source

```bash
cargo run --release
```


## ComputerUse

Plugovr implements a ComputerUse Interface using Qwen2.5VL-7B. 

spin up your local llm server for Qwen2.5VL-7B

```bash
vllm serve "Qwen/Qwen2.5-VL-7B-Instruct"
```

then run PlugOvr with the computeruse feature

```bash
cargo run --release --features computeruse
```

There are two shortcuts defined for computeruse:

F4: to get a dialog to enter the instruction

F2: to proceed the next action

THIS IS A BETA FEATURE AND MIGHT NOT WORK AS EXPECTED.

This is why we have no AUTO mode for computeruse yet. 

Its all about getting experience about the capabilities of Qwen2.5VL-7B.
