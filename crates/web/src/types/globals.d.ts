// Side-effect CSS imports (`import './globals.css'`) carry no type
// information of their own. Next generates a reference for this in
// `next-env.d.ts`, but that file is regenerated on every build and is
// gitignored, so the declaration lives here instead — it has to be
// present for a cold `bun run type-check` to pass.
declare module "*.css";
