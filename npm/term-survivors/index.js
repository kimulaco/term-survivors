#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");
const { platform, arch } = process;

const PLATFORMS = {
  darwin: {
    arm64: "@term-survivors/darwin-arm64/term-survivors",
    x64: "@term-survivors/darwin-x64/term-survivors",
  },
  linux: {
    x64: "@term-survivors/linux-x64/term-survivors",
    arm64: "@term-survivors/linux-arm64/term-survivors",
  },
  win32: {
    x64: "@term-survivors/win32-x64/term-survivors.exe",
  },
};

const binPath = PLATFORMS?.[platform]?.[arch];

if (!binPath) {
  console.error(
    `term-survivors: unsupported platform ${platform}-${arch}.\n` +
      `Supported: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64`
  );
  process.exit(1);
}

let bin;
try {
  bin = require.resolve(binPath);
} catch {
  console.error(
    `term-survivors: platform package for ${platform}-${arch} is not installed.\n` +
      `Try: npm install --include=optional`
  );
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), {
  shell: false,
  stdio: "inherit",
  env: {
    ...process.env,
    TERM_SURVIVORS_INSTALLED_VIA: "npm",
  },
});

if (result.error) {
  throw result.error;
}

process.exitCode = result.status;
