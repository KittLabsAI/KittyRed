import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);

if (args.length === 0) {
  console.error("Missing python script path.");
  process.exit(1);
}

const candidates = [];
if (process.env.PYTHON) {
  candidates.push(process.env.PYTHON);
}
candidates.push("python3", "python");

for (const command of candidates) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (!result.error) {
    process.exit(result.status ?? 0);
  }
}

console.error(`Unable to find a usable Python command. Tried: ${candidates.join(", ")}`);
process.exit(1);
