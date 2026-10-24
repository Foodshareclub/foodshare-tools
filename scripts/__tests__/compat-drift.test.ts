import { mkdir, mkdtemp, rm } from "bun:fs/promises";
import { tmpdir } from "bun:os";
import { dirname, join } from "bun:path";
import { expect, test } from "bun:test";

async function inspect(change: "backend-only" | "vendored-code" | "missing-path") {
  const root = await mkdtemp(join(tmpdir(), "foodshare-compat-drift-"));
  const server = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: () => new Response("healthy") });
  try {
    const scripts = join(root, "scripts");
    await mkdir(scripts);
    await Bun.write(join(scripts, "compat-check.sh"), Bun.file(new URL("../compat-check.sh", import.meta.url)));
    // CI selection is tested independently; this fixture isolates drift and
    // the shell report without network credentials or external API calls.
    await Bun.write(join(scripts, "compat-ci.ts"), 'console.log("| CI fixture | PASS | Fixture pipeline |");');
    for (const repo of ["web", "backend"]) {
      const supabase = join(root, repo, "supabase");
      await mkdir(join(supabase, "functions"), { recursive: true });
      await mkdir(join(supabase, "migrations"));
      await Bun.write(join(supabase, "config.toml"), 'project_id = "fixture"\n');
      await Bun.write(join(supabase, "seed.sql"), "-- seed fixture\n");
      await Bun.write(join(supabase, "migrations/001.sql"), "-- migration fixture\n");
      await Bun.write(join(supabase, "functions/index.ts"), "export {};\n");
    }
    if (change === "backend-only") {
      for (const path of ["oracle", "tests", "functions/node_modules"]) {
        await mkdir(join(root, "backend/supabase", path), { recursive: true });
        await Bun.write(join(root, "backend/supabase", path, "fixture.txt"), "backend-only fixture");
      }
    } else if (change === "vendored-code") {
      await Bun.write(join(root, "web/supabase/functions/index.ts"), "export const drift = true;\n");
    } else {
      await rm(join(root, "web/supabase/config.toml"));
    }
    const child = Bun.spawn(["bash", join(scripts, "compat-check.sh")], {
      env: {
        ...Bun.env,
        PATH: `${dirname(process.execPath)}:${Bun.env.PATH ?? ""}`,
        WEB_DIR: join(root, "web"),
        BACKEND_DIR: join(root, "backend"),
        API_HEALTH: server.url.toString(),
        STUDIO_URL: server.url.toString(),
        GITHUB_STEP_SUMMARY: join(root, "summary.md"),
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [code, output, error] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    expect(error).toBe("");
    return { code, output };
  } finally {
    await server.stop(true);
    await rm(root, { recursive: true, force: true });
  }
}

test("backend-only harnesses and runtime dependencies do not create web snapshot drift", async () => {
  const result = await inspect("backend-only");
  expect(result.code).toBe(0);
  expect(result.output).toContain("Vendored copy drift | PASS");
});

test("changed vendored function code still fails compatibility", async () => {
  const result = await inspect("vendored-code");
  expect(result.code).toBe(1);
  expect(result.output).toContain("supabase/functions diverged");
});

test("missing required configuration cannot pass as zero differences", async () => {
  const result = await inspect("missing-path");
  expect(result.code).toBe(1);
  expect(result.output).toContain("Missing required vendored path: config.toml");
});
