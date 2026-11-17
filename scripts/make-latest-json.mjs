// 生成 Tauri updater 的 latest.json（多平台）。
//
// 用法：node scripts/make-latest-json.mjs <artifactsDir> [--tag v1.2.2]
//   --tag 指定更新包所在的 Release 标签（默认 v<package.json 版本>）
//
// 识别规则：同目录下同名 `<资产>.sig` 存在才认为该平台可更新，
// 缺签名的平台直接跳过（宁可不提示，也不发一个装不上的更新）。
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const dir = args[0] && !args[0].startsWith("--") ? args[0] : "artifacts";
const tagIdx = args.indexOf("--tag");
const conf = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const version = conf.version;
const tag = tagIdx >= 0 && args[tagIdx + 1] ? args[tagIdx + 1] : `v${version}`;

/** 资产文件名 → Tauri 平台标识 */
const CANDIDATES = [
  { platform: "darwin-aarch64", file: `HGhub_${version}_arm64.app.tar.gz` },
  { platform: "darwin-x86_64", file: `HGhub_${version}_x64.app.tar.gz` },
  { platform: "windows-aarch64", file: `HGhub_${version}_arm64_setup.exe` },
  { platform: "windows-x86_64", file: `HGhub_${version}_x64_setup.exe` },
  { platform: "linux-aarch64", file: `HGhub_${version}_arm64.AppImage` },
  { platform: "linux-x86_64", file: `HGhub_${version}_amd64.AppImage` },
];

const base = `https://github.com/Kepler16f/HGhub/releases/download/${tag}`;
const platforms = {};

for (const { platform, file } of CANDIDATES) {
  const asset = path.join(dir, file);
  const sig = `${asset}.sig`;
  if (!fs.existsSync(asset) || !fs.existsSync(sig)) {
    console.warn(`[latest.json] 跳过 ${platform}：缺少 ${file} 或签名`);
    continue;
  }
  platforms[platform] = {
    signature: fs.readFileSync(sig, "utf8").trim(),
    url: `${base}/${file}`,
  };
}

if (Object.keys(platforms).length === 0) {
  console.error("[latest.json] 没有任何带签名的更新包，拒绝生成");
  process.exit(1);
}

const out = {
  version,
  notes: `HGhub ${version}`,
  pub_date: new Date().toISOString(),
  platforms,
};
fs.writeFileSync(path.join(dir, "latest.json"), `${JSON.stringify(out, null, 2)}\n`);
console.log(`[latest.json] ${path.join(dir, "latest.json")} → ${Object.keys(platforms).join(", ")}`);
