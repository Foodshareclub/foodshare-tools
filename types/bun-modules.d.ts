/**
 * Ambient declarations for Bun's `bun:`-prefixed builtin modules.
 *
 * `@types/bun` declares `bun:test`, `bun:ffi`, `bun:jsc`, `bun:sqlite` and
 * `bun:bundle` — but not the filesystem, path, os, util, crypto and DNS
 * modules, even though Bun implements every one of them. Without these
 * declarations an import of `bun:path` fails the type-check with
 *
 *     error TS2307: Cannot find module 'bun:path' or its corresponding type
 *                  declarations.
 *
 * Each declaration re-exports the `node:` module it mirrors, so the two
 * spellings share one set of types and cannot drift apart: `bun:path` and
 * `node:path` are the same API, and the only difference between them is which
 * runtime resolves them.
 *
 * The exports are written as `typeof import("node:path").join` rather than as
 * a re-export (`export { join } from "node:path"`). That matters: `node:fs`
 * and `node:path` declare heavily overloaded functions (`readdirSync`,
 * `statSync`, `globSync`, `join`, …), and a plain re-export collapses each
 * overload set to its last signature — which silently turns callback
 * parameters into `any` at every call site. `typeof import(...)` carries every
 * overload across intact.
 *
 * The types come from `@types/node`, which `@types/bun` depends on
 * transitively. That is a types-only dependency: it contributes no runtime
 * code and no Node.js binary.
 *
 * The same file is carried by the backend, the app and the tools
 * repositories. It is a copy rather than a shared package because each of
 * them is checked on its own, and a checker that cannot resolve `bun:path`
 * cannot see the rest of the tree either.
 */

/**
 * Bun's path module. Mirrors `node:path`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:path" {
  export const basename: typeof import("node:path").basename;
  export const delimiter: typeof import("node:path").delimiter;
  export const dirname: typeof import("node:path").dirname;
  export const extname: typeof import("node:path").extname;
  export const format: typeof import("node:path").format;
  export const isAbsolute: typeof import("node:path").isAbsolute;
  export const join: typeof import("node:path").join;
  export const matchesGlob: typeof import("node:path").matchesGlob;
  export const normalize: typeof import("node:path").normalize;
  export const parse: typeof import("node:path").parse;
  export const relative: typeof import("node:path").relative;
  export const resolve: typeof import("node:path").resolve;
  export const sep: typeof import("node:path").sep;
  export const toNamespacedPath: typeof import("node:path").toNamespacedPath;
  export type PlatformPath = import("node:path").PlatformPath;
  export type FormatInputPathObject = import("node:path").FormatInputPathObject;
  export type ParsedPath = import("node:path").ParsedPath;
}

/**
 * Bun's synchronous filesystem module. Mirrors `node:fs`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:fs" {
  export const constants: typeof import("node:fs").constants;
  export const copyFile: typeof import("node:fs").copyFile;
  export const copyFileSync: typeof import("node:fs").copyFileSync;
  export const cp: typeof import("node:fs").cp;
  export const cpSync: typeof import("node:fs").cpSync;
  export const createReadStream: typeof import("node:fs").createReadStream;
  export const createWriteStream: typeof import("node:fs").createWriteStream;
  export const existsSync: typeof import("node:fs").existsSync;
  export const glob: typeof import("node:fs").glob;
  export const globSync: typeof import("node:fs").globSync;
  export const lchmod: typeof import("node:fs").lchmod;
  export const lchmodSync: typeof import("node:fs").lchmodSync;
  export const lstat: typeof import("node:fs").lstat;
  export const lstatSync: typeof import("node:fs").lstatSync;
  export const lutimes: typeof import("node:fs").lutimes;
  export const lutimesSync: typeof import("node:fs").lutimesSync;
  export const mkdir: typeof import("node:fs").mkdir;
  export const mkdirSync: typeof import("node:fs").mkdirSync;
  export const mkdtemp: typeof import("node:fs").mkdtemp;
  export const mkdtempSync: typeof import("node:fs").mkdtempSync;
  export const open: typeof import("node:fs").open;
  export const openSync: typeof import("node:fs").openSync;
  export const opendir: typeof import("node:fs").opendir;
  export const opendirSync: typeof import("node:fs").opendirSync;
  export const readFile: typeof import("node:fs").readFile;
  export const readFileSync: typeof import("node:fs").readFileSync;
  export const readdir: typeof import("node:fs").readdir;
  export const readdirSync: typeof import("node:fs").readdirSync;
  export const readlink: typeof import("node:fs").readlink;
  export const readlinkSync: typeof import("node:fs").readlinkSync;
  export const realpath: typeof import("node:fs").realpath;
  export const realpathSync: typeof import("node:fs").realpathSync;
  export const rename: typeof import("node:fs").rename;
  export const renameSync: typeof import("node:fs").renameSync;
  export const rm: typeof import("node:fs").rm;
  export const rmSync: typeof import("node:fs").rmSync;
  export const rmdir: typeof import("node:fs").rmdir;
  export const rmdirSync: typeof import("node:fs").rmdirSync;
  export const stat: typeof import("node:fs").stat;
  export const statSync: typeof import("node:fs").statSync;
  export const symlink: typeof import("node:fs").symlink;
  export const symlinkSync: typeof import("node:fs").symlinkSync;
  export const truncate: typeof import("node:fs").truncate;
  export const truncateSync: typeof import("node:fs").truncateSync;
  export const unlink: typeof import("node:fs").unlink;
  export const unlinkSync: typeof import("node:fs").unlinkSync;
  export const utimes: typeof import("node:fs").utimes;
  export const utimesSync: typeof import("node:fs").utimesSync;
  export const watch: typeof import("node:fs").watch;
  export const watchFile: typeof import("node:fs").watchFile;
  export const writeFile: typeof import("node:fs").writeFile;
  export const writeFileSync: typeof import("node:fs").writeFileSync;
  export const Dir: typeof import("node:fs").Dir;
  export const Dirent: typeof import("node:fs").Dirent;
  export const Stats: typeof import("node:fs").Stats;
  export const StatsFs: typeof import("node:fs").StatsFs;
  export const ReadStream: typeof import("node:fs").ReadStream;
  export const WriteStream: typeof import("node:fs").WriteStream;
  // `Dirent` and `Stats` are classes. `import { type Dirent }` needs the name
  // to be a *type* too, and a `const` declaration only provides the value —
  // so the type is exported separately under the same name.
  export type Dirent = import("node:fs").Dirent;
  export type Stats = import("node:fs").Stats;
  export type StatsFs = import("node:fs").StatsFs;
  export type ReadStream = import("node:fs").ReadStream;
  export type WriteStream = import("node:fs").WriteStream;
  export type PathLike = import("node:fs").PathLike;
  export type PathOrFileDescriptor = import("node:fs").PathOrFileDescriptor;
  export type GlobOptions = import("node:fs").GlobOptions;
  export type GlobOptionsWithFileTypes = import("node:fs").GlobOptionsWithFileTypes;
  export type GlobOptionsWithoutFileTypes = import("node:fs").GlobOptionsWithoutFileTypes;
  export type RmDirOptions = import("node:fs").RmDirOptions;
  export type RmOptions = import("node:fs").RmOptions;
  export type WatchOptions = import("node:fs").WatchOptions;
}

/**
 * Bun's promise-based filesystem module. Mirrors `node:fs/promises`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:fs/promises" {
  export const appendFile: typeof import("node:fs/promises").appendFile;
  export const chmod: typeof import("node:fs/promises").chmod;
  export const chown: typeof import("node:fs/promises").chown;
  export const copyFile: typeof import("node:fs/promises").copyFile;
  export const cp: typeof import("node:fs/promises").cp;
  export const lchmod: typeof import("node:fs/promises").lchmod;
  export const lstat: typeof import("node:fs/promises").lstat;
  export const lutimes: typeof import("node:fs/promises").lutimes;
  export const mkdir: typeof import("node:fs/promises").mkdir;
  export const mkdtemp: typeof import("node:fs/promises").mkdtemp;
  export const open: typeof import("node:fs/promises").open;
  export const opendir: typeof import("node:fs/promises").opendir;
  export const readFile: typeof import("node:fs/promises").readFile;
  export const readdir: typeof import("node:fs/promises").readdir;
  export const readlink: typeof import("node:fs/promises").readlink;
  export const realpath: typeof import("node:fs/promises").realpath;
  export const rename: typeof import("node:fs/promises").rename;
  export const rm: typeof import("node:fs/promises").rm;
  export const rmdir: typeof import("node:fs/promises").rmdir;
  export const stat: typeof import("node:fs/promises").stat;
  export const symlink: typeof import("node:fs/promises").symlink;
  export const truncate: typeof import("node:fs/promises").truncate;
  export const unlink: typeof import("node:fs/promises").unlink;
  export const utimes: typeof import("node:fs/promises").utimes;
  export const watch: typeof import("node:fs/promises").watch;
  export const writeFile: typeof import("node:fs/promises").writeFile;
  export type FileHandle = import("node:fs/promises").FileHandle;
  export type RmDirOptions = import("node:fs/promises").RmDirOptions;
  export type RmOptions = import("node:fs/promises").RmOptions;
}

/**
 * Bun's util module. Mirrors `node:util`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:util" {
  export const parseArgs: typeof import("node:util").parseArgs;
  export const inspect: typeof import("node:util").inspect;
  export const format: typeof import("node:util").format;
  export const promisify: typeof import("node:util").promisify;
  export const types: typeof import("node:util").types;
}

/**
 * Bun's promise-based DNS module. Mirrors `node:dns/promises`.
 *
 * The guard in `scripts/check-no-node.ts` maps `node:dns/promises` to this
 * spelling, so a file that follows the guard has to be able to resolve it.
 */
declare module "bun:dns/promises" {
  export const lookup: typeof import("node:dns/promises").lookup;
  export const resolve: typeof import("node:dns/promises").resolve;
  export const resolve4: typeof import("node:dns/promises").resolve4;
  export const resolve6: typeof import("node:dns/promises").resolve6;
  export const resolveAny: typeof import("node:dns/promises").resolveAny;
  export const resolveCname: typeof import("node:dns/promises").resolveCname;
  export const resolveMx: typeof import("node:dns/promises").resolveMx;
  export const resolveNaptr: typeof import("node:dns/promises").resolveNaptr;
  export const resolveNs: typeof import("node:dns/promises").resolveNs;
  export const resolveSrv: typeof import("node:dns/promises").resolveSrv;
  export const resolveTxt: typeof import("node:dns/promises").resolveTxt;
  export const reverse: typeof import("node:dns/promises").reverse;
  export const setServers: typeof import("node:dns/promises").setServers;
}

/**
 * Bun's operating-system module. Mirrors `node:os`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:os" {
  export const arch: typeof import("node:os").arch;
  export const cpus: typeof import("node:os").cpus;
  export const devNull: typeof import("node:os").devNull;
  export const endianness: typeof import("node:os").endianness;
  export const freemem: typeof import("node:os").freemem;
  export const getPriority: typeof import("node:os").getPriority;
  export const homedir: typeof import("node:os").homedir;
  export const hostname: typeof import("node:os").hostname;
  export const loadavg: typeof import("node:os").loadavg;
  export const machine: typeof import("node:os").machine;
  export const networkInterfaces: typeof import("node:os").networkInterfaces;
  export const platform: typeof import("node:os").platform;
  export const release: typeof import("node:os").release;
  export const setPriority: typeof import("node:os").setPriority;
  export const tmpdir: typeof import("node:os").tmpdir;
  export const totalmem: typeof import("node:os").totalmem;
  export const type: typeof import("node:os").type;
  export const uptime: typeof import("node:os").uptime;
  export const userInfo: typeof import("node:os").userInfo;
  export const version: typeof import("node:os").version;
  export const EOL: typeof import("node:os").EOL;
  export type CpuInfo = import("node:os").CpuInfo;
  export type Dict = import("node:os").Dict;
  export type NodeJS = import("node:os").NodeJS;
  export type NetworkInterfaceInfo = import("node:os").NetworkInterfaceInfo;
  export type OS = import("node:os").OS;
  export type UserInfo = import("node:os").UserInfo;
}

/**
 * Bun's cryptography module. Mirrors `node:crypto`.
 *
 * Bun implements this module itself; `@types/bun` declares no ambient module
 * for it, so the import fails to type-check without this declaration.
 */
declare module "bun:crypto" {
  export const createHash: typeof import("node:crypto").createHash;
  export const createHmac: typeof import("node:crypto").createHmac;
  export const createPrivateKey: typeof import("node:crypto").createPrivateKey;
  export const createPublicKey: typeof import("node:crypto").createPublicKey;
  export const createSign: typeof import("node:crypto").createSign;
  export const createVerify: typeof import("node:crypto").createVerify;
  export const getRandomValues: typeof import("node:crypto").getRandomValues;
  export const randomBytes: typeof import("node:crypto").randomBytes;
  export const randomFillSync: typeof import("node:crypto").randomFillSync;
  export const randomInt: typeof import("node:crypto").randomInt;
  export const randomUUID: typeof import("node:crypto").randomUUID;
  export const subtle: typeof import("node:crypto").subtle;
  export const timingSafeEqual: typeof import("node:crypto").timingSafeEqual;
  export type BinaryLike = import("node:crypto").BinaryLike;
  export type Hash = import("node:crypto").Hash;
  export type Hmac = import("node:crypto").Hmac;
}

/**
 * Every other `bun:` spelling.
 *
 * The guard in `scripts/check-no-node.ts` rewrites a Node builtin to its
 * `bun:` spelling, and its table is an exception list rather than the
 * rule: anything outside the table becomes `bun:<name>` as well. So the
 * declarations have to cover the general case, or the guard's advice
 * fails the type check for every builtin that is not in the table --
 * `bun:zlib`, `bun:http`, `bun:stream`, and the rest.
 *
 * This declaration is the general case. It gives the module the type
 * `any`, which is what an undeclared import would have failed the build
 * over; the precise declarations above win over it, so a module that is
 * declared properly keeps its real types. TypeScript prefers an exact
 * module match over a pattern match, and `bun:path` still rejects
 * `join(1, 2)`.
 */
declare module "bun:*";
