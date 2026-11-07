# Google Inactive OAuth Clients — Complete Record

Date: 2026-11-03  
Working directory: `/Users/organic/dev/work/google` (currently empty; used as the session workspace)  
Session context: user asked, after receiving Google’s deletion warning, “can we address these issues?”  
Status: **Plan mode** — no repo changes made yet; this document is the record requested by the user.

---

## 1. What the user requested

1. The user pasted a Google Developers email received at 2:26 AM (and a near‑duplicate at 2:47 AM) notifying them that several projects they manage have OAuth 2.0 clients inactive for at least five months. Inactive clients would be deleted in 30 days; active clients in the same projects would not be affected.
2. The first email listed six projects:
   - `audiobookshelf-449600`
   - `crea-479506`
   - `foodshare-71b57`
   - `topdesignpro76`
   - `hybridized-325021`
   - `careecho`
3. The second email omitted `audiobookshelf-449600`, implying Google now sees that client as active.
4. The user wanted to address these deletions and explored keeping the clients alive.
5. When asked how to retain clients, the user chose **“Probe anyway”** (an automated GitHub Actions probe) over “Real retention only” or “Just delete everything.”
6. When asked which projects are actually used, the user answered: **all of the existing projects** — specifically `audiobookshelf`, `crea`, `foodshare`, `hybridized`, `careecho`, `topdesignpro76`, and all existing projects.
7. The user authorized reading only `project_id` / `client_id` from local client‑secret files to map them to projects. **No secret material was printed, copied, or transmitted.**
8. The user asked to **fully document** the request, findings, and chosen approach — covering everything.

---

## 2. What the notification says

- In accordance with OAuth 2.0 Policies, OAuth clients inactive for at least five months will be deleted in **30 days** unless action is taken.
- Active clients in the same projects will not be affected.
- To prevent deletion, either:
  - use the client in a **Sign in with Google** flow, or
  - **exchange a refresh token for an access token**.
- If nothing is done, clients are automatically deleted. Once deleted, you have **up to 30 days from deletion** to restore them via Google Cloud Console.
- This is a service‑information notice about the Auth Platform project, sent by Google LLC.

**Important dates**
- Notice received: 2026-11-03
- Deletion window: on/after **≈ 2026-12-03**
- Post‑deletion restore window: until **≈ 2027-01-02** (30 days after deletion)

---

## 3. Environment and inventory findings

### 3.1 Tooling / access
- `gcloud` is not installed / not on `PATH`.
- `rg` is not installed; `find`/`grep` were used with pruning.
- `gh` **is** installed at `/opt/homebrew/bin/gh`, authenticated as `organicnz` (GitHub.com), token scopes include `repo`, `workflow`, `admin:org`, `delete_repo`. This means a GitHub Actions workflow can legitimately be created, but placement still needs user approval.
- Administrative access to Google Cloud Console has not been established or attempted.

### 3.2 Local references to the flagged projects
Searched (read‑only, no secrets printed):
- `/Users/organic/dev/work/audiobookshelf`
- `/Users/organic/dev/work/careecho`
- `/Users/organic/dev/work/crea`
- `/Users/organic/dev/work/foodshare`
- `/Users/organic/dev/work/hybridized`
- `/Users/organic/dev/work/desingup` and `desingup-web`
- `/Users/organic/dev/work/creds/Google`
- `/Users/organic/dev/work/google` (empty)

**OAuth client IDs found in source/config (only public client IDs were printed):**

| Client ID (project number) | Found in | Likely project |
|---|---|---|
| `370204032392-590c55mgkqfhe3t40suas8ckld1sgnnr...` | `audiobookshelf/docker-compose-backup.yml`, `audiobookshelf/enable-oauth.sql` | `audiobookshelf-449600` |
| `137491020140-g5i69nfvo7fafq0hlqu050hv80iv097a...` | `crea/.env.local` | `crea-479506` |
| `314120228176-39uqgge6e7vlmchmahh24a64pdmt7aiq...` | `creds/Google/web/client_secret_*.json` (keys: `web`) | `foodshare-71b57` |
| `314120228176-jkeegcv3gpsei98sc6d3gqiscf7glp3f...` | `foodshare/foodshare-backend/.github/workflows/*.yml`, deploy logs, artifacts | `foodshare-71b57` |
| `551126928286-dloaha01vbjkf07dqmh55iet868uhdbe...` | `foodshare/keys/PlayMarket/GCP/service account/client_secret_*.json` | `api-7264403823346633298-790360` |

Notes:
- `foodshare-71b57` has **two** Google web OAuth clients visible locally in different contexts: one in `creds/` and one used by foodshare‑backend CI/deploy.
- The Play Console project (`api-7264403823346633298-790360`) is **not** in the purge list and should not be touched as part of this task.
- No Google client IDs were found in `careecho`, `hybridized`, `desingup`, or the Google Play service‑account JSON.
- Large trees (`node_modules`, `.git`, `.next`, `dist`, `build`, `Pods`) were excluded from searches; files larger than 2 MB were also skipped.

### 3.3 Refresh tokens
- **No refresh tokens were located locally.** The two Google web `client_secret_*.json` files contain client credentials but **no refresh token**.
- The Play service‑account JSON is a different credential class and cannot be used for user‑token refresh.
- Therefore, a refresh‑token probe is **not immediately possible** for any client without locating (or re‑issuing) such a grant.

### 3.4 Local OAuth JSONs mapped (minimal fields only)
- `creds/Google/web/client_secret_314120228176-39uq...apps.googleusercontent.com.json`
  - top‑level key: `web`
  - `project_id`: `foodshare-71b57`
  - `client_id`: `314120228176-39uqgge6e7vlmchmahh24a64pdmt7aiq.apps.googleusercontent.com`
- `foodshare/keys/PlayMarket/GCP/service account/client_secret_551126928286-dloaha...json`
  - top‑level key: `web`
  - `project_id`: `api-7264403823346633298-790360`
  - `client_id`: `551126928286-dloaha01vbjkf07dqmh55iet868uhdbe.apps.googleusercontent.com`
- No other `client_secret*.json`, `oauth*.json`, `credentials*.json`, or `token*.json` files were found in the searched trees.

### 3.5 Google Cloud inventory (not yet obtained)
- The complete list of inactive OAuth clients **per project** is only visible in Cloud Console and has not been retrieved.
- Publishing status (**Production** vs **Testing**) of each OAuth consent screen is also unknown. This is a hard blocker for a refresh‑token probe:
  - In **Testing** mode, refresh tokens for unverified apps expire after ~7 days, so any probe would fail quickly.
  - In **Production** mode, a refresh‑token exchange is a valid, ongoing retention mechanism.

---

## 4. Feasibility of the options the user chose

### 4.1 “All projects are actually used”
The user says each of these projects has real usage. That is plausible for:
- `audiobookshelf-449600` (self‑hosted, with Caddy/oauth2‑proxy)
- `foodshare-71b57` (production backend / web sign‑in)
- `crea-479506` (development/web app; client ID present)
- `hybridized-325021` and `careecho` (likely via Supabase Auth with a Google provider configured in the dashboards)

It is unverified for:
- `topdesignpro76` — no matching local directory and no local client ID found.
- Any project whose only Google integration is in the Supabase dashboard; the repo would not contain a client ID.

### 4.2 “Probe anyway”
A probe can only **exchange a refresh token for an access token**. It cannot:
- perform a Sign in with Google flow headlessly (would require automating user consent);
- create a refresh token from a `client_secret` alone;
- keep alive web sign‑in clients that never requested offline access.

A GitHub Actions workflow **can** be built legitimately for clients that already hold refresh tokens:
- scheduled monthly (or quarterly) job;
- sends `grant_type=refresh_token` to `https://oauth2.googleapis.com/token`;
- logs only the client **name** and pass/fail; never prints tokens;
- on failure, opens a GitHub issue (no secrets) prompting re‑consent.

However, this is only viable if:
1. The user can name the stored refresh tokens and where they live (e.g. 1Password, Supabase, server environment, CI secrets, backend configuration), **and**
2. The consent screen is in **Production** (or otherwise not time‑limited), **and**
3. A private GitHub repository (or dedicated workflow in an existing private repo) holds the secrets.

Absent those, the honest outcomes are:
- one manual “Sign in with Google” per web client (valid for five months per client), or
- letting unused clients be deleted and restored later if needed.

### 4.3 Honesty / policy constraints
- Google’s notice explicitly lists refresh‑token exchange as a retention path, so using it with **genuine stored grants** is compliant with the notice.
- The following are **not** acceptable and will not be built:
  - scripted browser automation to simulate user sign‑ins;
  - forged tokens or fabricated “usage”;
  - any message to Google misrepresenting the nature of the integrations;
  - attempting to hide activity or circumvent the purpose of the cleanup.
- Unused clients should simply be allowed to be deleted; deletion is reversible for 30 days afterward.

---

## 5. Chosen approach

The user selected **“Probe anyway”** and stated all projects are genuinely used. The chosen approach, made precise by technical constraints, is:

**Phase 0 — Obtain the authoritative list (required, manual)**  
Open Google Cloud Console for each project and record:
- project ID;
- OAuth client name and client ID;
- client type (Web application / iOS / Android / Desktop / Service account);
- consent‑screen **publishing status** (Production vs Testing).

**Phase 1 — Classify each inactive client**  
For every client listed in Console:
- **(a) Web sign‑in clients** (likely `audiobookshelf`, `crea`, `hybridized`, `careecho`, and possibly `foodshare`)
  → Keep alive by completing **one real Google sign‑in** on the corresponding deployed app before the deletion date, and at least once every five months thereafter.
- **(b) Offline/API clients with a stored refresh token** (possibly one foodshare client, if it was ever issued one)
  → Probe‑keepalive with monthly token exchange.
- **(c) Unknown or dormant** (likely `topdesignpro76`)
  → Do nothing; allow deletion; restore from Console within 30 days if actually needed later.

**Phase 2 — Probe implementation (only if ≥ one (b) client exists)**  
Create or designate a **private** GitHub repository (recommended: new private repo, e.g. `organicnz/google-oauth-keepalive`), containing:

- `.github/workflows/keepalive.yml`
  - schedule: monthly (e.g. `0 8 1 * *`) plus `workflow_dispatch`;
  - runner: `ubuntu-latest`;
  - no marketplace dependencies beyond `actions/*` if avoidable;
  - script: bash/curl, because the job is a single OAuth token request;
  - secrets: one `GOOGLE_OAUTH_CLIENTS` JSON array, or one secret per client:
    `[{“name”:“foodshare-backend”,“client_id”:“...”,“client_secret”:“...”,“refresh_token”:“...”}]`;
  - output: client name and success/failure only;
  - failure path: open a GitHub issue titled `Keepalive failed: <name>` containing **no secrets** and requesting re‑consent;
  - success path: exit 0 with no token output.
- `README.md` with a rotation/runbook section (how to re‑issue a refresh token, how to update the secret, how to verify manually with `curl`).
- Optionally, a **no‑secrets reminder workflow** that opens an issue every ~4 months listing the web sign‑in clients that need a manual sign‑in.

**Phase 3 — Web sign‑in clients**  
No code is required. Execute a real sign‑in per client in the corresponding app, then verify in Console that the client no longer appears in the inactive list (or at least that activity was recorded). Set a recurring reminder every ~4 months.

**Phase 4 — Safety net**  
If any client is deleted on/after 2026‑12‑03, use Console to restore it within 30 days. Document the mapping from project → client → app URL so restoration is unambiguous.

**Phase 5 — Decommission instead of retain when appropriate**  
Any client that cannot be re‑consented, has no stored refresh grant, and is not actually required by a deployed app should be **allowed to delete**, not kept alive artificially.

---

## 6. Exact action checklist

Do not check these off until they are done.

- [x] Read the Google notification emails and record missing/duplicated project lists (audiobookshelf present in first only).
- [x] Inventory local directories and public client IDs (no secrets).
- [x] Map local client‑secret JSON files to projects (minimal fields only).
- [x] Confirm `gcloud` is unavailable; `gh` is available and authenticated.
- [ ] Open Google Cloud Console → APIs & Services → Credentials for each of the six projects.
- [ ] Record for each project: affected OAuth clients, their types, and consent‑screen publishing status.
- [ ] Locate or confirm **no existing refresh tokens** for flagged clients.
- [ ] Classify clients as (a), (b), or (c).
- [ ] For each (a) client: perform one and only one real “Sign in with Google” on the deployed app.
- [ ] For each (b) client: ideally create the private keepalive repo/workflow, else do a one‑off token exchange.
- [ ] For each (c) client: explicitly decide **do not retain** and document the decision.
- [ ] Add/verify in Console that clients are no longer flagged as inactive.
- [ ] Record final status and restore deadline in project notes.

---

## 7. Risks and constraints

1. **No gcloud; Console is manual.** All inventory must be obtained by the user or via browser.
2. **No client_secret for most projects.** The only local web client secrets found belong to `foodshare-71b57` (and the non‑flagged Play project). Careecho, hybridized, crea, and audiobookshelf likely have client secrets elsewhere (env, Vercel, Supabase, server config).
3. **Supabase‑hosted Google OAuth hides the client from the repo.** The OAuth client remains in GCP, but the app never stores a Google refresh token; only real user sign‑ins register activity.
4. **Testing mode kills the probe.** Any client whose consent screen is still in Testing status will have refresh tokens expire in 7 days, making a monthly keepalive useless.
5. **A probe cannot spoof sign‑ins.** It can only use legitimately stored refresh tokens. False compliance is out of scope and not supported.
6. **Secrets in GitHub Actions are high‑risk.** The keepalive repo must be private, single‑collaborator or minimal‑trust, and the secret must be rotatable. Anyone with read access to logs or repo‑write access can exploit them.
7. **`topdesignpro76` has no local evidence.** It may be abandoned; ask the user before attempting anything.
8. **Deletion is not catastrophic.** If a client is deleted and later found necessary, it can be restored within 30 days.

---

## 8. Timeline / deadline

- Today: 2026-11-03
- Opportunity to retain: until **about 2026-12-03**
- Deletion expected: on/after that date, unless clients are used in a sanctioned way beforehand.
- Restoration available: up to 30 days after deletion, i.e. until roughly **2027-01-02**.

---

## 9. Open questions needed to proceed

The user’s answers so far have been recorded. The remaining items block implementation:

1. Console list for each project (client name/type/status) — paste or describe.
2. Publishing status of each consent screen (Production vs Testing).
3. Exact location(s) of any stored refresh tokens for:
   - `foodshare-71b57` (most likely candidate),
   - any other flagged project.
4. Whether Google Cloud Console shows the inactive clients and can be used to identify them.
5. Preferred placement for keepalive automation (new private repo vs existing private repo vs none).
6. Confirmation that `topdesignpro76` is actually needed and which Google app uses it.

---

## 10. Summary of decisions made

- The user wants to **address all six flagged projects** (not delete anything ahead of time).
- The user selected **“Probe anyway”** for keep‑alive, not “Real retention only” and not “delete everything.”
- The user states **all named projects are actually used**.
- Local inventory found **no refresh tokens** and **no client secrets for five of six projects**.
- The only viable automated retention is therefore the **refresh‑token exchange for any genuinely stored grants** (Phase 2). Web sign‑in retention (Phase 3) and do‑nothing/delete (Phase 1c) remain the honest alternatives for the rest.
- **No plan to misrepresent activity to Google.** The approach must remain a truthful record of either real usage or the retention path explicitly offered by Google’s notice.

---

## 11. Restore procedure (if deletion occurs)

Do **not** discover after the fact that this is needed; note it now:
1. Open Google Cloud Console.
2. Navigate to the relevant project → APIs & Services → Credentials (or the Deleted/Restore section shown after the deletion date).
3. Restore the affected OAuth client(s) within 30 days.
4. Note that restoring makes them available again, but prior tokens may have been invalidated; test sign‑in after restore.

Do not attempt restore outside the Console UI or by recreation with the same client ID (client IDs cannot be recreated with identical values).

---

## 12. Document status / limitations

- This is an internal planning and record‑keeping document, not legal advice and not a Google support commitment.
- It does not authorize any network call, file modification outside this document, or Google Cloud change without explicit user approval.
- The information in Section 3 will become stale as soon as Console is checked or code deploys occur.
- Before implementing any workflow, the open questions in Section 9 must be answered.
