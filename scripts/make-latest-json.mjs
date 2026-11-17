// 生成 Tauri updater 的 latest.json。
// 用法：node scripts/make-latest-json.mjs <artifactsDir> [version]
// 输入：HGhub_<version>_<arch>.app.tar.gz 与其 .sig（由 tauri build 的 updater 产物拷入）
// 输出：<artifactsDir>/latest.json
import fs from "node:fs";
import path from "node:path";

const dir = process.argv[2] || "artifacts";
const conf = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const version = process.argv[3] || conf.version;
const archMap = { arm64: "darwin-aarch64", x64: "darwin-x86_64" };
const base = `https://github.com/Kepler16f/HGhub/releases/download/v${version}-macos`;
const platforms = {};

for (const [arch, target] of Object.entries(archMap)) {
  const file = path.join(dir, `HGhub_${version}_${arch}.app.tar.gz`);
  const sigFile = `${file}.sig`;
  if (!fs.existsSync(file) || !fs.existsSync(sigFile)) {
    console.warn(`[latest.json] 跳过 ${target}：缺少 ${path.basename(file)} 或签名`);
    continue;
  }
  platforms[target] = {
    signature: fs.readFileSync(sigFile, "utf8").trim(),
    url: `${base}/HGhub_${version}_${arch}.app.tar.gz`,
  };
}

if (Object.keys(platforms).length === 0) {
  console.error("[latest.json] 没有任何签名更新包，拒绝生成");
  process.exit(1);
}

const out = {
  version,
  notes: `HGhub ${version} macOS 更新`,
  pub_date: new Date().toISOString(),
  platforms,
};
fs.writeFileSync(path.join(dir, "latest.json"), `${JSON.stringify(out, null, 2)}\n`);
console.log(`[latest.json] ${path.join(dir, "latest.json")} → ${Object.keys(platforms).join(", ")}`);
