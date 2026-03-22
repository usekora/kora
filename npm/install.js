const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const REPO = "kora-ai/kora";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "linux-x64": "x86_64-unknown-linux-gnu",
};

function install() {
  const platform = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[platform];

  if (!target) {
    console.error(`Unsupported platform: ${platform}`);
    process.exit(1);
  }

  const version = require("./package.json").version;
  const url = `https://github.com/${REPO}/releases/download/v${version}/kora-${target}.tar.gz`;

  console.log(`Downloading kora v${version} for ${target}...`);

  fs.mkdirSync(BIN_DIR, { recursive: true });

  const tmpFile = path.join(BIN_DIR, "kora.tar.gz");
  execFileSync("curl", ["-fsSL", "-o", tmpFile, url], { stdio: "inherit" });
  execFileSync("tar", ["xzf", tmpFile, "-C", BIN_DIR], { stdio: "inherit" });
  fs.unlinkSync(tmpFile);
  fs.chmodSync(path.join(BIN_DIR, "kora"), 0o755);

  console.log("kora installed successfully");
}

install();
