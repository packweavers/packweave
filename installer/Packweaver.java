package packweaver;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URI;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.security.CodeSource;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

public class Packweaver {
    private static final String UA = "packweave-installer/1.0.0 (+https://github.com/packweavers/packweave)";
    private static final String STATE_FILE = ".packweaver-state.json";
    private static final String WORK_DIR = ".packweaver";
    private static final String RELEASE_PREFIX = "release+";
    private static final AtomicBoolean RAN = new AtomicBoolean();

    public static void main(String[] args) {
        String env = "all";
        boolean finish = false;
        boolean releaseFlag = false;
        boolean tokenNext = false;
        String url = null;
        String token = null;
        Path dir = null;
        for (String raw : args) {
            String a = raw.replace("\"", "").replace("'", "").trim();
            if (a.isEmpty()) {
                continue;
            }
            if (tokenNext) {
                token = a;
                tokenNext = false;
            } else if (a.equals("--server")) {
                env = "server";
            } else if (a.equals("--client")) {
                env = "client";
            } else if (a.equals("--finish")) {
                finish = true;
            } else if (a.equals("--token")) {
                tokenNext = true;
            } else if (a.startsWith("--token=")) {
                token = a.substring("--token=".length());
            } else if (a.startsWith("github_pat_") || a.startsWith("ghp_")) {
                token = a;
            } else if (a.equals(RELEASE_PREFIX)) {
                releaseFlag = true;
            } else if (a.startsWith("http://") || a.startsWith("https://")
                    || a.startsWith(RELEASE_PREFIX)) {
                url = a;
            } else {
                dir = Paths.get(a);
            }
        }
        if (releaseFlag && url != null && !url.startsWith(RELEASE_PREFIX)) {
            url = RELEASE_PREFIX + url;
        }
        if (dir == null) {
            dir = Paths.get(".").toAbsolutePath().normalize();
        }
        if (finish) {
            try {
                finishStage(dir, 200);
            } catch (Exception e) {
                System.err.println("packweaver: " + e.getMessage());
            }
            return;
        }
        try {
            Files.createDirectories(dir);
            token = cleanToken(token);
            if (token != null) {
                Files.write(dir.resolve("modpack.token"),
                        (token + "\n").getBytes(StandardCharsets.UTF_8));
                System.out.println("packweaver: token saved");
            }
            if (url == null) {
                url = readUrl(dir);
            }
            if (url == null || url.trim().isEmpty()) {
                if ("server".equals(env)) {
                    throw new IOException("No pack URL provided.");
                }
                url = prompt("Pack URL: ");
            }
            if (url == null || url.trim().isEmpty()) {
                throw new IOException("No pack URL provided.");
            }
            run(dir, url, env, false);
        } catch (Exception e) {
            System.err.println("packweaver: " + e.getMessage());
            System.exit(1);
        }
    }

    public static void premain(String agentArgs) {
        try {
            entry(false);
        } catch (Throwable t) {
            System.err.println("packweaver: sync skipped: " + t.getMessage());
        }
    }

    public static void modEntry() {
        try {
            entry(true);
        } catch (Throwable t) {
            System.err.println("packweaver: sync skipped: " + t.getMessage());
        }
    }

    private static void entry(boolean inGame) throws Exception {
        if (!RAN.compareAndSet(false, true)) {
            return;
        }
        if (System.getProperty("packweaver.synced") != null) {
            return;
        }
        System.setProperty("packweaver.synced", "1");
        Path dir = findGameDir();
        String url = dir == null ? null : readUrl(dir);
        if (url == null || url.trim().isEmpty()) {
            System.out.println("packweaver: no pack URL found, skipping sync");
            return;
        }
        run(dir, url, inGame && detectServer() ? "server" : "all", inGame);
    }

    private static void run(Path dir, String url, String env, boolean inGame) throws Exception {
        url = url.replace("\uFEFF", "").replace("\"", "").replace("'", "").trim();
        if (url.startsWith(RELEASE_PREFIX)) {
            url = RELEASE_PREFIX
                    + url.substring(RELEASE_PREFIX.length()).trim();
        }
        for (int i = 0; i < url.length(); i++) {
            if (Character.isWhitespace(url.charAt(i))) {
                url = url.substring(0, i);
                break;
            }
        }
        url = url.replaceAll("/+$", "");
        if (!url.startsWith("http://") && !url.startsWith("https://")
                && !url.startsWith(RELEASE_PREFIX)) {
            throw new IOException("Invalid pack URL: " + url);
        }
        try {
            finishStage(dir, 1);
        } catch (Exception ignore) {
        }
        Path urlFile = dir.resolve("modpack.url");
        String want = url + "\n";
        if (!Files.exists(urlFile) || !want.equals(new String(Files.readAllBytes(urlFile), StandardCharsets.UTF_8))) {
            Files.write(urlFile, want.getBytes(StandardCharsets.UTF_8));
        }
        if (url.startsWith(RELEASE_PREFIX)) {
            runRelease(dir, url.substring(RELEASE_PREFIX.length()), env, inGame);
        } else {
            runCommit(dir, url, env, inGame);
        }
    }

    private static void runCommit(Path dir, String url, String env, boolean inGame) throws Exception {
        try {
            Map<String, Object> mp = asMap(Json.parse(get(url + "/modpack.json")));
            String mc = str(mp.get("minecraft"));
            String loader = str(mp.get("loader"));
            System.out.println("packweaver: syncing from " + url
                    + (mc != null ? " (Minecraft " + mc + (loader != null ? " / " + loader : "") + ")" : ""));
        } catch (Exception e) {
            System.out.println("packweaver: syncing from " + url);
        }

        Map<String, Object> index = asMap(Json.parse(get(url + "/index.json")));
        List<String> contentPaths = new ArrayList<String>();
        List<String[]> tasks = new ArrayList<String[]>();
        for (Object eo : asList(index.get("files"))) {
            Map<String, Object> f = asMap(eo);
            String path = str(f.get("path"));
            if (path == null || path.isEmpty()) {
                continue;
            }
            if (path.startsWith("content/") && path.endsWith(".json")) {
                contentPaths.add(path);
            } else if (path.startsWith("overrides/")) {
                String inst = path.substring("overrides/".length());
                if (!inst.isEmpty()) {
                    tasks.add(new String[] { inst, url + "/" + enc(path), str(f.get("sha1")) });
                }
            }
        }

        final String base = url;
        final String srv = env;
        ExecutorService metaPool = Executors.newFixedThreadPool(8);
        List<Future<String[]>> metaFutures = new ArrayList<Future<String[]>>();
        for (final String cp : contentPaths) {
            metaFutures.add(metaPool.submit(new java.util.concurrent.Callable<String[]>() {
                public String[] call() {
                    try {
                        Map<String, Object> m = asMap(Json.parse(get(base + "/" + enc(cp))));
                        return contentTask(cp, m, srv);
                    } catch (Exception e) {
                        System.err.println("  ! " + cp + ": " + e.getMessage());
                        return null;
                    }
                }
            }));
        }
        for (Future<String[]> ft : metaFutures) {
            String[] t = ft.get();
            if (t != null) {
                tasks.add(t);
            }
        }
        metaPool.shutdown();

        List<Pattern> preserve;
        try {
            preserve = parsePreserve(get(url + "/preserve.txt"));
        } catch (Exception e) {
            preserve = new ArrayList<Pattern>();
        }
        apply(dir, tasks, Collections.<String, byte[]>emptyMap(), preserve, inGame, null, null,
                false);
    }

    private static void runRelease(Path dir, String webUrl, String env, boolean inGame) throws Exception {
        System.out.println("packweaver: syncing from " + webUrl);
        String token = readToken(dir);
        String[] or = ownerRepo(webUrl);
        String repoApi = apiBase(webUrl) + "/repos/" + or[0] + "/" + or[1];
        String[] meta = loadMeta(dir);
        String lastTag = meta[0];
        ApiResp resp = apiGet(repoApi + "/releases/latest", token, lastTag != null ? meta[1] : null);
        if (resp.code == 304) {
            verifyLocal(dir, repoApi, token, env, inGame, lastTag);
            return;
        }
        if (resp.code == 404) {
            throw new IOException("No releases found for " + or[0] + "/" + or[1] + ".");
        }
        Map<String, Object> rel = asMap(Json.parse(resp.body));
        String tag = str(rel.get("tag_name"));
        if (tag == null || tag.isEmpty()) {
            throw new IOException("Malformed release data.");
        }
        Map<String, String> assets = new LinkedHashMap<String, String>();
        for (Object ao : asList(rel.get("assets"))) {
            Map<String, Object> a = asMap(ao);
            String n = str(a.get("name"));
            String u = str(a.get("url"));
            if (n != null && u != null) {
                assets.put(n, u);
            }
        }
        if (tag.equals(lastTag)) {
            saveState(dir, loadState(dir), tag, resp.etag, null);
            verifyLocal(dir, repoApi, token, env, inGame, tag);
            return;
        }
        System.out.println("packweaver: updating to " + tag);
        String packUrl = assets.get("pack.zip");
        String deltaName = lastTag == null ? null : "delta-" + lastTag + ".zip";
        boolean usedDelta = false;
        Map<String, byte[]> entries;
        if (deltaName != null && assets.containsKey(deltaName)) {
            entries = unzip(fetchBytes(assets.get(deltaName), token, "application/octet-stream"));
            usedDelta = true;
        } else {
            if (packUrl == null) {
                throw new IOException("Release " + tag + " has no pack.zip asset.");
            }
            entries = unzip(fetchBytes(packUrl, token, "application/octet-stream"));
        }
        SyncInput in = buildFromZip(entries, env);
        if (usedDelta && needsFull(dir, in)) {
            if (packUrl == null) {
                throw new IOException("Release " + tag + " has no pack.zip asset.");
            }
            entries = unzip(fetchBytes(packUrl, token, "application/octet-stream"));
            in = buildFromZip(entries, env);
        }
        byte[] newToken = entries.get("modpack.token");
        if (newToken != null) {
            String t = cleanToken(new String(newToken, StandardCharsets.UTF_8));
            if (t != null && !t.equals(token)) {
                Files.write(dir.resolve("modpack.token"), (t + "\n").getBytes(StandardCharsets.UTF_8));
            }
        }
        apply(dir, in.tasks, in.local, in.preserve, inGame, tag, resp.etag, false);
    }

    private static void verifyLocal(Path dir, String repoApi, String token, String env,
            boolean inGame, String tag) throws Exception {
        Map<String, String> state = loadState(dir);
        Map<String, String> stats = loadStats(dir);
        List<String> needs = new ArrayList<String>();
        boolean dirty = false;
        for (Map.Entry<String, String> e : state.entrySet()) {
            String rel = e.getKey();
            Path p = dir.resolve(rel);
            if (!Files.isRegularFile(p)) {
                if (Files.isRegularFile(dir.resolve(rel + ".disabled"))) {
                    continue;
                }
                needs.add(rel);
                continue;
            }
            if (!contentPath(rel)) {
                continue;
            }
            String cur;
            try {
                cur = Files.size(p) + "," + Files.getLastModifiedTime(p).toMillis();
            } catch (Exception ex) {
                cur = "";
            }
            if (cur.equals(stats.get(rel))) {
                continue;
            }
            String disk = diskSha(p);
            if (disk != null && disk.equalsIgnoreCase(e.getValue())) {
                stats.put(rel, cur);
                dirty = true;
                continue;
            }
            needs.add(rel);
        }
        if (needs.isEmpty()) {
            if (dirty) {
                saveState(dir, state, null, null, stats);
            }
            System.out.println("packweaver: up to date (" + tag + ")");
            return;
        }
        System.out.println("packweaver: repairing " + needs.size()
                + (needs.size() == 1 ? " file" : " files"));
        ApiResp resp = apiGet(repoApi + "/releases/latest", token, null);
        if (resp.body == null) {
            throw new IOException("Couldn't reach the release to repair files.");
        }
        Map<String, Object> rel = asMap(Json.parse(resp.body));
        Map<String, String> assets = new LinkedHashMap<String, String>();
        for (Object ao : asList(rel.get("assets"))) {
            Map<String, Object> a = asMap(ao);
            String n = str(a.get("name"));
            String u = str(a.get("url"));
            if (n != null && u != null) {
                assets.put(n, u);
            }
        }
        String packUrl = assets.get("pack.zip");
        if (packUrl == null) {
            throw new IOException("Release has no pack.zip asset.");
        }
        Map<String, byte[]> entries = unzip(fetchBytes(packUrl, token, "application/octet-stream"));
        SyncInput in = buildFromZip(entries, env);
        List<String[]> tasks = new ArrayList<String[]>();
        for (String[] t : in.tasks) {
            if (needs.contains(t[0])) {
                tasks.add(t);
            }
        }
        apply(dir, tasks, in.local, in.preserve, inGame, null, null, true);
    }

    private static boolean contentPath(String rel) {
        return rel.startsWith("mods/") || rel.startsWith("resourcepacks/")
                || rel.startsWith("shaderpacks/");
    }

    private static final class SyncInput {
        private final List<String[]> tasks = new ArrayList<String[]>();
        private final Map<String, byte[]> local = new HashMap<String, byte[]>();
        private List<Pattern> preserve = new ArrayList<Pattern>();
    }

    private static SyncInput buildFromZip(Map<String, byte[]> entries, String env) throws IOException {
        SyncInput in = new SyncInput();
        byte[] idx = entries.get("index.json");
        if (idx == null) {
            throw new IOException("Release is missing index.json.");
        }
        Map<String, Object> index = asMap(Json.parse(new String(idx, StandardCharsets.UTF_8)));
        for (Object eo : asList(index.get("files"))) {
            Map<String, Object> f = asMap(eo);
            String path = str(f.get("path"));
            if (path == null || path.isEmpty()) {
                continue;
            }
            if (path.startsWith("content/") && path.endsWith(".json")) {
                byte[] cj = entries.get(path);
                if (cj == null) {
                    System.err.println("  ! " + path + ": missing from release");
                    continue;
                }
                String[] t = contentTask(path,
                        asMap(Json.parse(new String(cj, StandardCharsets.UTF_8))), env);
                if (t != null) {
                    in.tasks.add(t);
                }
            } else if (path.startsWith("overrides/")) {
                String inst = path.substring("overrides/".length());
                if (inst.isEmpty()) {
                    continue;
                }
                in.tasks.add(new String[] { inst, null, str(f.get("sha1")) });
                byte[] bytes = entries.get(path);
                if (bytes != null) {
                    in.local.put(inst, bytes);
                }
            }
        }
        byte[] pres = entries.get("preserve.txt");
        if (pres != null) {
            in.preserve = parsePreserve(new String(pres, StandardCharsets.UTF_8));
        }
        return in;
    }

    private static boolean needsFull(Path dir, SyncInput in) {
        for (String[] t : in.tasks) {
            if (t[1] != null || in.local.containsKey(t[0])) {
                continue;
            }
            String want = t[2];
            String disk = diskSha(dir.resolve(t[0]));
            if (want == null || disk == null || !want.equalsIgnoreCase(disk)) {
                return true;
            }
        }
        return false;
    }

    private static String[] contentTask(String cp, Map<String, Object> m, String env) {
        Map<String, Object> src = asMap(asMap(m.get("sources")).get(str(m.get("preferred"))));
        String dl = str(src.get("downloadUrl"));
        String fn = str(src.get("filename"));
        if (dl == null || dl.isEmpty() || fn == null || fn.isEmpty()) {
            return null;
        }
        if ("server".equals(env) && "unsupported".equals(str(m.get("serverSide")))) {
            return null;
        }
        if ("client".equals(env) && "unsupported".equals(str(m.get("clientSide")))) {
            return null;
        }
        String[] segs = cp.split("/");
        String sub = segs.length >= 2 ? segs[1] : "mods";
        String folder = "shaders".equals(sub) ? "shaderpacks" : sub;
        return new String[] { folder + "/" + fn, dl, str(src.get("sha1")) };
    }

    private static int apply(Path dir, List<String[]> tasksIn, final Map<String, byte[]> local,
            final List<Pattern> preserve, boolean inGame, String tag, String etag, boolean partial)
            throws Exception {
        LinkedHashMap<String, String> desired = new LinkedHashMap<String, String>();
        List<String[]> downloads = new ArrayList<String[]>();
        for (String[] t : tasksIn) {
            String rel = safeRel(t[0]);
            if (rel == null) {
                System.err.println("  ! skipped unsafe path: " + t[0]);
                continue;
            }
            t[0] = rel;
            if (protectedPath(t[0]) || desired.containsKey(t[0])) {
                continue;
            }
            desired.put(t[0], t[2]);
            downloads.add(t);
        }

        Map<String, String> state = loadState(dir);
        final Map<String, String> newState = partial
                ? new ConcurrentHashMap<String, String>(state)
                : new ConcurrentHashMap<String, String>();
        final Map<String, String> newStats =
                new ConcurrentHashMap<String, String>(loadStats(dir));
        final List<String[]> pending = Collections.synchronizedList(new ArrayList<String[]>());
        final AtomicInteger modsChanged = new AtomicInteger();

        for (Map.Entry<String, String> e : partial
                ? Collections.<String, String>emptyMap().entrySet()
                : state.entrySet()) {
            String old = e.getKey();
            if (desired.containsKey(old)) {
                continue;
            }
            if (protectedPath(old) || ownJarPath(old) || safeRel(old) == null) {
                continue;
            }
            Path p = dir.resolve(old);
            String disk = diskSha(p);
            if (disk == null) {
                try {
                    Files.deleteIfExists(dir.resolve(old + ".disabled"));
                } catch (Exception ignore) {
                }
            }
            if (disk != null && matches(preserve, old) && !disk.equalsIgnoreCase(e.getValue())) {
                System.out.println("  ~ kept " + old + " (edited locally)");
                continue;
            }
            if (disk != null && old.startsWith("mods/")) {
                modsChanged.incrementAndGet();
            }
            if (disk != null && inGame && old.startsWith("mods/")) {
                pending.add(new String[] { "del", old, null, null });
                newState.put(old, e.getValue());
                continue;
            }
            try {
                Files.deleteIfExists(p);
                newStats.remove(old);
            } catch (Exception ex) {
                pending.add(new String[] { "del", old, null, null });
                newState.put(old, e.getValue());
            }
        }

        final Path root = dir;
        final boolean game = inGame;
        final Map<String, String> prevState = state;
        final String ownRel = ownJarRel();
        ExecutorService pool = Executors.newFixedThreadPool(8);
        List<Future<?>> futures = new ArrayList<Future<?>>();
        final int total = downloads.size();
        final AtomicInteger done = new AtomicInteger();
        final AtomicInteger failed = new AtomicInteger();
        final AtomicInteger kept = new AtomicInteger();
        for (final String[] d : downloads) {
            futures.add(pool.submit(new Runnable() {
                public void run() {
                    final String rel = d[0];
                    final String src = d[1];
                    final String sha1 = d[2];
                    try {
                        Path out = root.resolve(rel);
                        String disk = diskSha(out);
                        String wrote = prevState.get(rel);
                        if (sha1 != null && !sha1.isEmpty() && disk != null && sha1.equalsIgnoreCase(disk)) {
                            newState.put(rel, disk.toLowerCase());
                            recordStat(newStats, root, rel);
                            done.incrementAndGet();
                            return;
                        }
                        if (disk == null
                                && Files.isRegularFile(root.resolve(rel + ".disabled"))) {
                            if (wrote != null) {
                                newState.put(rel, wrote);
                            }
                            done.incrementAndGet();
                            System.out.println("  ~ skipped " + rel + " (disabled)");
                            return;
                        }
                        if (disk != null && matches(preserve, rel)
                                && (wrote == null || !wrote.equalsIgnoreCase(disk))) {
                            if (wrote != null) {
                                newState.put(rel, wrote);
                            }
                            kept.incrementAndGet();
                            System.out.println("  ~ kept " + rel + " (edited locally)");
                            return;
                        }
                        byte[] pre = local.get(rel);
                        byte[] bytes;
                        if (pre != null) {
                            bytes = pre;
                        } else if (src != null) {
                            bytes = getBytes(src);
                        } else {
                            throw new IOException("no data in release");
                        }
                        String got = sha1Of(bytes);
                        if (sha1 != null && !sha1.isEmpty() && !sha1.equalsIgnoreCase(got)) {
                            throw new IOException("checksum mismatch");
                        }
                        if (rel.startsWith("mods/")) {
                            modsChanged.incrementAndGet();
                        }
                        boolean stage = (game && rel.startsWith("mods/") && disk != null)
                                || rel.equals(ownRel) || ownJarPath(rel);
                        if (!stage) {
                            try {
                                Files.createDirectories(out.getParent());
                                Files.write(out, bytes);
                                newState.put(rel, got);
                                recordStat(newStats, root, rel);
                                int n = done.incrementAndGet();
                                System.out.println("  [" + n + "/" + total + "] " + rel);
                                return;
                            } catch (IOException locked) {
                                stage = true;
                            }
                        }
                        Path sf = stageFile(root, rel);
                        Files.createDirectories(sf.getParent());
                        Files.write(sf, bytes);
                        pending.add(new String[] { "put", rel, got, root.relativize(sf).toString().replace('\\', '/') });
                        if (wrote != null) {
                            newState.put(rel, wrote);
                        }
                        int n = done.incrementAndGet();
                        System.out.println("  [" + n + "/" + total + "] " + rel + " (queued)");
                    } catch (Exception e) {
                        failed.incrementAndGet();
                        System.err.println("  ! " + rel + ": " + e.getMessage());
                    }
                }
            }));
        }
        for (Future<?> f : futures) {
            f.get();
        }
        pool.shutdown();

        boolean ok = failed.get() == 0;
        saveState(dir, newState, ok ? tag : null, ok ? etag : null, newStats);

        if (!pending.isEmpty()) {
            savePending(dir, pending);
            scheduleHelper(dir);
            System.out.println("packweaver: " + pending.size()
                    + (pending.size() == 1 ? " change finishes" : " changes finish") + " after the game closes");
        }

        if (failed.get() > 0) {
            System.out.println("Done with " + failed.get() + " failures, " + newState.size()
                    + " files tracked in " + dir);
        } else {
            System.out.println("Up to date, " + newState.size() + " files in " + dir
                    + (kept.get() > 0 ? " (" + kept.get() + " kept)" : ""));
        }
        if (inGame && modsChanged.get() > 0) {
            System.out.println("packweaver: mods updated, closing the game. start it again to play");
            System.exit(0);
        }
        return failed.get();
    }

    private static void recordStat(Map<String, String> stats, Path root, String rel) {
        try {
            Path p = root.resolve(rel);
            stats.put(rel, Files.size(p) + "," + Files.getLastModifiedTime(p).toMillis());
        } catch (Exception ignore) {
        }
    }

    private static String readUrl(Path dir) throws IOException {
        Path urlFile = dir.resolve("modpack.url");
        if (Files.exists(urlFile)) {
            return new String(Files.readAllBytes(urlFile), StandardCharsets.UTF_8).trim();
        }
        return null;
    }

    private static String readToken(Path dir) {
        try {
            Path f = dir.resolve("modpack.token");
            if (Files.exists(f)) {
                return cleanToken(new String(Files.readAllBytes(f), StandardCharsets.UTF_8));
            }
        } catch (Exception ignore) {
        }
        return null;
    }

    private static String cleanToken(String t) {
        if (t == null) {
            return null;
        }
        t = t.replace("\uFEFF", "").replace("\"", "").replace("'", "").trim();
        for (int i = 0; i < t.length(); i++) {
            if (Character.isWhitespace(t.charAt(i))) {
                t = t.substring(0, i);
                break;
            }
        }
        return t.isEmpty() ? null : t;
    }

    private static String[] ownerRepo(String webUrl) throws IOException {
        List<String> segs = new ArrayList<String>();
        try {
            for (String s : new URL(webUrl).getPath().split("/")) {
                if (!s.isEmpty()) {
                    segs.add(s);
                }
            }
        } catch (Exception e) {
            throw new IOException("Release URL must look like https://github.com/you/pack");
        }
        if (segs.size() < 2) {
            throw new IOException("Release URL must look like https://github.com/you/pack");
        }
        String owner = segs.get(segs.size() - 2);
        String repo = segs.get(segs.size() - 1);
        if (repo.endsWith(".git")) {
            repo = repo.substring(0, repo.length() - 4);
        }
        return new String[] { owner, repo };
    }

    private static String apiBase(String webUrl) throws IOException {
        URL u = new URL(webUrl);
        String auth = u.getAuthority();
        if (auth.equals("github.com") || auth.equals("www.github.com")) {
            return "https://api.github.com";
        }
        return u.getProtocol() + "://" + auth + "/api/v3";
    }

    private static final class ApiResp {
        private int code;
        private String body;
        private String etag;
    }

    private static ApiResp apiGet(String url, String token, String etag) throws IOException {
        HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
        c.setConnectTimeout(15000);
        c.setReadTimeout(60000);
        c.setRequestProperty("User-Agent", UA);
        c.setRequestProperty("Accept", "application/vnd.github+json");
        c.setRequestProperty("X-GitHub-Api-Version", "2022-11-28");
        if (token != null && !token.isEmpty()) {
            c.setRequestProperty("Authorization", "Bearer " + token);
        }
        if (etag != null && !etag.isEmpty()) {
            c.setRequestProperty("If-None-Match", etag);
        }
        ApiResp r = new ApiResp();
        r.code = c.getResponseCode();
        r.etag = c.getHeaderField("ETag");
        if (r.code / 100 == 2) {
            InputStream in = c.getInputStream();
            try {
                r.body = new String(readAll(in), StandardCharsets.UTF_8);
            } finally {
                in.close();
            }
        } else {
            InputStream err = c.getErrorStream();
            if (err != null) {
                try {
                    readAll(err);
                } finally {
                    err.close();
                }
            }
            if (r.code != 304 && r.code != 404) {
                throw new IOException("HTTP " + r.code + " for " + url);
            }
        }
        return r;
    }

    private static boolean protectedPath(String rel) {
        return rel.equals("modpack.url") || rel.equals("modpack.token") || rel.equals(STATE_FILE)
                || rel.startsWith(WORK_DIR + "/");
    }

    private static String safeRel(String rel) {
        if (rel == null) {
            return null;
        }
        String r = rel.replace('\\', '/').trim();
        while (r.startsWith("/")) {
            r = r.substring(1);
        }
        if (r.isEmpty() || r.indexOf(':') == 1 || r.contains("://")) {
            return null;
        }
        StringBuilder sb = new StringBuilder();
        for (String part : r.split("/")) {
            String p = part.trim();
            if (p.isEmpty() || p.equals(".")) {
                continue;
            }
            if (p.equals("..")) {
                return null;
            }
            if (sb.length() > 0) {
                sb.append('/');
            }
            sb.append(p);
        }
        return sb.length() == 0 ? null : sb.toString();
    }

    private static boolean ownJarPath(String rel) {
        return rel.equals("packweaver.jar") || rel.equals("mods/packweaver.jar");
    }

    private static String ownJarRel() {
        Path jar = ownJar();
        if (jar == null || jar.getFileName() == null) {
            return "packweaver.jar";
        }
        Path parent = jar.getParent();
        String name = jar.getFileName().toString();
        if (parent != null && parent.getFileName() != null && parent.getFileName().toString().equals("mods")) {
            return "mods/" + name;
        }
        return name;
    }

    private static Path stageFile(Path dir, String rel) throws Exception {
        return dir.resolve(WORK_DIR).resolve("stage").resolve(sha1Of(rel.getBytes(StandardCharsets.UTF_8)) + ".bin");
    }

    private static void savePending(Path dir, List<String[]> ops) throws IOException {
        StringBuilder sb = new StringBuilder("{\"ops\":[");
        boolean first = true;
        for (String[] op : ops) {
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append("{\"op\":").append(Json.quote(op[0]));
            sb.append(",\"path\":").append(Json.quote(op[1]));
            if (op[2] != null) {
                sb.append(",\"sha\":").append(Json.quote(op[2]));
            }
            if (op[3] != null) {
                sb.append(",\"stage\":").append(Json.quote(op[3]));
            }
            sb.append('}');
        }
        sb.append("]}\n");
        Path f = dir.resolve(WORK_DIR).resolve("pending.json");
        Files.createDirectories(f.getParent());
        atomicWrite(f, sb.toString().getBytes(StandardCharsets.UTF_8));
    }

    private static void finishStage(Path dir, int rounds) throws Exception {
        Path pf = dir.resolve(WORK_DIR).resolve("pending.json");
        if (!Files.exists(pf)) {
            return;
        }
        List<String[]> ops = new ArrayList<String[]>();
        Map<String, Object> parsed = asMap(Json.parse(new String(Files.readAllBytes(pf), StandardCharsets.UTF_8)));
        for (Object oo : asList(parsed.get("ops"))) {
            Map<String, Object> o = asMap(oo);
            ops.add(new String[] { str(o.get("op")), str(o.get("path")), str(o.get("sha")), str(o.get("stage")) });
        }
        Map<String, String> state = loadState(dir);
        for (int r = 0; r < rounds && !ops.isEmpty(); r++) {
            if (r > 0) {
                Thread.sleep(300);
            }
            List<String[]> remaining = new ArrayList<String[]>();
            for (String[] op : ops) {
                try {
                    if (safeRel(op[1]) == null) {
                        state.remove(op[1]);
                        continue;
                    }
                    Path target = dir.resolve(op[1]);
                    if ("del".equals(op[0])) {
                        Files.deleteIfExists(target);
                        state.remove(op[1]);
                    } else {
                        Path sf = op[3] == null || safeRel(op[3]) == null
                                ? null
                                : dir.resolve(op[3]);
                        if (sf == null || !Files.exists(sf)) {
                            continue;
                        }
                        Files.createDirectories(target.getParent());
                        Files.move(sf, target, StandardCopyOption.REPLACE_EXISTING);
                        if (op[2] != null) {
                            state.put(op[1], op[2]);
                        }
                    }
                } catch (Exception e) {
                    remaining.add(op);
                }
            }
            ops = remaining;
        }
        saveState(dir, state, null, null, null);
        if (ops.isEmpty()) {
            Files.deleteIfExists(pf);
            Path stage = dir.resolve(WORK_DIR).resolve("stage");
            if (Files.isDirectory(stage)) {
                for (Path p : listFiles(stage)) {
                    Files.deleteIfExists(p);
                }
                Files.deleteIfExists(stage);
            }
        } else {
            savePending(dir, ops);
        }
    }

    private static List<Path> listFiles(Path dir) throws IOException {
        List<Path> out = new ArrayList<Path>();
        java.nio.file.DirectoryStream<Path> ds = Files.newDirectoryStream(dir);
        try {
            for (Path p : ds) {
                out.add(p);
            }
        } finally {
            ds.close();
        }
        return out;
    }

    private static void scheduleHelper(final Path dir) {
        Runtime.getRuntime().addShutdownHook(new Thread(new Runnable() {
            public void run() {
                try {
                    Path jar = ownJar();
                    if (jar == null) {
                        return;
                    }
                    boolean win = System.getProperty("os.name", "").toLowerCase().contains("win");
                    Path bin = Paths.get(System.getProperty("java.home"), "bin", win ? "javaw.exe" : "java");
                    if (!Files.exists(bin)) {
                        bin = Paths.get(System.getProperty("java.home"), "bin", win ? "java.exe" : "java");
                    }
                    Path log = dir.resolve(WORK_DIR).resolve("finish.log");
                    Files.createDirectories(log.getParent());
                    new ProcessBuilder(bin.toString(), "-cp", jar.toString(), "packweaver.Packweaver", "--finish",
                            dir.toString())
                            .redirectErrorStream(true)
                            .redirectOutput(log.toFile())
                            .start();
                } catch (Exception ignore) {
                }
            }
        }));
    }

    private static List<Pattern> parsePreserve(String text) {
        List<Pattern> out = new ArrayList<Pattern>();
        for (String line : text.split("\n")) {
            String t = line.trim();
            if (t.isEmpty() || t.startsWith("#")) {
                continue;
            }
            out.add(glob(t));
        }
        return out;
    }

    private static boolean matches(List<Pattern> preserve, String rel) {
        for (Pattern p : preserve) {
            if (p.matcher(rel).matches()) {
                return true;
            }
        }
        return false;
    }

    private static Pattern glob(String pat) {
        String p = pat.replace('\\', '/');
        while (p.startsWith("/")) {
            p = p.substring(1);
        }
        if (p.endsWith("/")) {
            p = p + "**";
        }
        StringBuilder sb = new StringBuilder();
        int i = 0;
        while (i < p.length()) {
            char c = p.charAt(i);
            if (c == '*') {
                if (i + 1 < p.length() && p.charAt(i + 1) == '*') {
                    sb.append(".*");
                    i += 2;
                } else {
                    sb.append("[^/]*");
                    i++;
                }
            } else if (c == '?') {
                sb.append("[^/]");
                i++;
            } else {
                if ("\\.[]{}()+-^$|".indexOf(c) >= 0) {
                    sb.append('\\');
                }
                sb.append(c);
                i++;
            }
        }
        return Pattern.compile(sb.toString());
    }

    private static Map<String, String> loadState(Path dir) {
        Map<String, String> out = new LinkedHashMap<String, String>();
        Path f = dir.resolve(STATE_FILE);
        if (!Files.exists(f)) {
            return out;
        }
        try {
            Map<String, Object> st = asMap(Json.parse(new String(Files.readAllBytes(f), StandardCharsets.UTF_8)));
            Object files = st.get("files");
            if (files instanceof Map) {
                for (Map.Entry<String, Object> e : asMap(files).entrySet()) {
                    String sha = str(e.getValue());
                    if (sha != null && !sha.isEmpty()) {
                        out.put(e.getKey(), sha.toLowerCase());
                    }
                }
            } else {
                for (Object p : asList(files)) {
                    String rel = str(p);
                    if (rel == null) {
                        continue;
                    }
                    String disk = diskSha(dir.resolve(rel));
                    if (disk != null) {
                        out.put(rel, disk);
                    }
                }
            }
        } catch (Exception ignore) {
        }
        return out;
    }

    private static Map<String, String> loadStats(Path dir) {
        Map<String, String> out = new LinkedHashMap<String, String>();
        Path f = dir.resolve(STATE_FILE);
        if (!Files.exists(f)) {
            return out;
        }
        try {
            Map<String, Object> st = asMap(Json.parse(new String(Files.readAllBytes(f), StandardCharsets.UTF_8)));
            for (Map.Entry<String, Object> e : asMap(st.get("stat")).entrySet()) {
                String v = str(e.getValue());
                if (v != null && !v.isEmpty()) {
                    out.put(e.getKey(), v);
                }
            }
        } catch (Exception ignore) {
        }
        return out;
    }

    private static String[] loadMeta(Path dir) {
        Path f = dir.resolve(STATE_FILE);
        if (!Files.exists(f)) {
            return new String[] { null, null };
        }
        try {
            Map<String, Object> st = asMap(Json.parse(new String(Files.readAllBytes(f), StandardCharsets.UTF_8)));
            return new String[] { str(st.get("tag")), str(st.get("etag")) };
        } catch (Exception ignore) {
            return new String[] { null, null };
        }
    }

    private static void saveState(Path dir, Map<String, String> state, String tag, String etag,
            Map<String, String> stats) throws IOException {
        if (tag == null) {
            String[] m = loadMeta(dir);
            tag = m[0];
            etag = m[1];
        }
        if (stats == null) {
            stats = loadStats(dir);
        }
        List<String> keys = new ArrayList<String>(state.keySet());
        Collections.sort(keys);
        StringBuilder sb = new StringBuilder("{\"version\":2");
        if (tag != null) {
            sb.append(",\"tag\":").append(Json.quote(tag));
        }
        if (etag != null) {
            sb.append(",\"etag\":").append(Json.quote(etag));
        }
        sb.append(",\"files\":{");
        boolean first = true;
        for (String k : keys) {
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append(Json.quote(k)).append(':').append(Json.quote(state.get(k)));
        }
        sb.append("},\"stat\":{");
        first = true;
        for (String k : keys) {
            String v = stats.get(k);
            if (v == null) {
                continue;
            }
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append(Json.quote(k)).append(':').append(Json.quote(v));
        }
        sb.append("}}\n");
        atomicWrite(dir.resolve(STATE_FILE), sb.toString().getBytes(StandardCharsets.UTF_8));
    }

    private static void atomicWrite(Path target, byte[] bytes) throws IOException {
        Path tmp = target.resolveSibling(target.getFileName() + ".tmp");
        Files.write(tmp, bytes);
        try {
            Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING,
                    StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException e) {
            Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING);
        }
    }

    private static String diskSha(Path p) {
        try {
            if (!Files.isRegularFile(p)) {
                return null;
            }
            return sha1Of(Files.readAllBytes(p));
        } catch (Exception e) {
            return null;
        }
    }

    private static boolean detectServer() {
        try {
            Class<?> fl = Class.forName("net.fabricmc.loader.api.FabricLoader");
            Object inst = fl.getMethod("getInstance").invoke(null);
            Object env = fl.getMethod("getEnvironmentType").invoke(inst);
            return "SERVER".equals(((Enum<?>) env).name());
        } catch (Throwable ignore) {
        }
        for (String cn : new String[] { "net.neoforged.fml.loading.FMLEnvironment",
                "net.minecraftforge.fml.loading.FMLEnvironment" }) {
            try {
                Object dist = Class.forName(cn).getField("dist").get(null);
                return "DEDICATED_SERVER".equals(((Enum<?>) dist).name());
            } catch (Throwable ignore) {
            }
        }
        return false;
    }

    private static Path findGameDir() {
        List<Path> cands = new ArrayList<Path>();
        Path jar = ownJar();
        if (jar != null && jar.getParent() != null) {
            Path pd = jar.getParent();
            if (pd.getFileName() != null && pd.getFileName().toString().equals("mods") && pd.getParent() != null) {
                cands.add(pd.getParent());
            } else {
                cands.add(pd);
            }
        }
        cands.add(Paths.get(".").toAbsolutePath().normalize());
        for (Path c : cands) {
            if (Files.exists(c.resolve("modpack.url"))) {
                return c;
            }
        }
        return null;
    }

    private static Path ownJar() {
        try {
            CodeSource cs = Packweaver.class.getProtectionDomain().getCodeSource();
            if (cs == null || cs.getLocation() == null) {
                return null;
            }
            String loc = cs.getLocation().toString();
            if (loc.startsWith("jar:")) {
                loc = loc.substring(4);
            }
            if (loc.startsWith("union:")) {
                loc = "file:" + loc.substring(6);
            }
            int bang = loc.indexOf('!');
            if (bang >= 0) {
                loc = loc.substring(0, bang);
            }
            int enc = loc.indexOf("%23");
            if (enc >= 0) {
                loc = loc.substring(0, enc);
            }
            int hash = loc.indexOf('#');
            if (hash >= 0) {
                loc = loc.substring(0, hash);
            }
            if (loc.startsWith("file:")) {
                return Paths.get(new URI(loc.replace(" ", "%20")));
            }
            return Paths.get(loc);
        } catch (Throwable t) {
            return null;
        }
    }

    private static String enc(String path) {
        StringBuilder sb = new StringBuilder();
        for (String seg : path.split("/")) {
            if (sb.length() > 0) {
                sb.append('/');
            }
            try {
                sb.append(URLEncoder.encode(seg, "UTF-8").replace("+", "%20"));
            } catch (java.io.UnsupportedEncodingException e) {
                sb.append(seg);
            }
        }
        return sb.toString();
    }

    private static Map<String, byte[]> unzip(byte[] data) throws IOException {
        Map<String, byte[]> out = new LinkedHashMap<String, byte[]>();
        ZipInputStream z = new ZipInputStream(new ByteArrayInputStream(data));
        try {
            ZipEntry e;
            while ((e = z.getNextEntry()) != null) {
                if (e.isDirectory()) {
                    continue;
                }
                out.put(e.getName(), readAll(z));
            }
        } finally {
            z.close();
        }
        return out;
    }

    private static byte[] readAll(InputStream in) throws IOException {
        ByteArrayOutputStream bo = new ByteArrayOutputStream();
        byte[] buf = new byte[16384];
        int n;
        while ((n = in.read(buf)) > 0) {
            bo.write(buf, 0, n);
        }
        return bo.toByteArray();
    }

    private static String get(String url) throws IOException {
        return new String(getBytes(url), StandardCharsets.UTF_8);
    }

    private static byte[] getBytes(String url) throws IOException {
        return fetchBytes(url, null, null);
    }

    private static byte[] fetchBytes(String url, String token, String accept) throws IOException {
        String current = url;
        String authHost;
        try {
            authHost = new URL(url).getAuthority();
        } catch (Exception e) {
            authHost = "";
        }
        for (int i = 0; i < 5; i++) {
            HttpURLConnection c = (HttpURLConnection) new URL(current).openConnection();
            c.setInstanceFollowRedirects(false);
            c.setConnectTimeout(15000);
            c.setReadTimeout(60000);
            c.setRequestProperty("User-Agent", UA);
            if (accept != null) {
                c.setRequestProperty("Accept", accept);
            }
            if (token != null && !token.isEmpty()
                    && new URL(current).getAuthority().equals(authHost)) {
                c.setRequestProperty("Authorization", "Bearer " + token);
            }
            int code = c.getResponseCode();
            if (code / 100 == 3) {
                String loc = c.getHeaderField("Location");
                c.disconnect();
                if (loc == null) {
                    throw new IOException("HTTP " + code + " for " + current);
                }
                current = new URL(new URL(current), loc).toString();
                continue;
            }
            if (code / 100 != 2) {
                c.disconnect();
                throw new IOException("HTTP " + code + " for " + current);
            }
            InputStream in = c.getInputStream();
            try {
                return readAll(in);
            } finally {
                in.close();
            }
        }
        throw new IOException("Too many redirects for " + url);
    }

    private static String sha1Of(byte[] b) throws Exception {
        byte[] h = MessageDigest.getInstance("SHA-1").digest(b);
        StringBuilder sb = new StringBuilder();
        for (byte x : h) {
            sb.append(String.format("%02x", x));
        }
        return sb.toString();
    }

    private static String prompt(String msg) throws IOException {
        if (System.console() != null) {
            return System.console().readLine(msg);
        }
        System.out.print(msg);
        return new BufferedReader(new InputStreamReader(System.in)).readLine();
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> asMap(Object o) {
        return o instanceof Map ? (Map<String, Object>) o : new HashMap<String, Object>();
    }

    @SuppressWarnings("unchecked")
    private static List<Object> asList(Object o) {
        return o instanceof List ? (List<Object>) o : new ArrayList<Object>();
    }

    private static String str(Object o) {
        return o == null ? null : o.toString();
    }

    private static final class Json {
        private final String s;
        private int i;

        private Json(String s) {
            this.s = s;
        }

        private static Object parse(String s) {
            Json j = new Json(s);
            return j.val();
        }

        private Object val() {
            ws();
            char c = s.charAt(i);
            switch (c) {
                case '{':
                    return obj();
                case '[':
                    return arr();
                case '"':
                    return strv();
                case 't':
                    i += 4;
                    return Boolean.TRUE;
                case 'f':
                    i += 5;
                    return Boolean.FALSE;
                case 'n':
                    i += 4;
                    return null;
                default:
                    return num();
            }
        }

        private Map<String, Object> obj() {
            Map<String, Object> m = new LinkedHashMap<String, Object>();
            i++;
            ws();
            if (s.charAt(i) == '}') {
                i++;
                return m;
            }
            while (true) {
                ws();
                String k = strv();
                ws();
                i++;
                Object v = val();
                m.put(k, v);
                ws();
                char c = s.charAt(i++);
                if (c == '}') {
                    break;
                }
            }
            return m;
        }

        private List<Object> arr() {
            List<Object> a = new ArrayList<Object>();
            i++;
            ws();
            if (s.charAt(i) == ']') {
                i++;
                return a;
            }
            while (true) {
                a.add(val());
                ws();
                char c = s.charAt(i++);
                if (c == ']') {
                    break;
                }
            }
            return a;
        }

        private String strv() {
            StringBuilder sb = new StringBuilder();
            i++;
            while (true) {
                char c = s.charAt(i++);
                if (c == '"') {
                    break;
                }
                if (c == '\\') {
                    char e = s.charAt(i++);
                    switch (e) {
                        case '"':
                            sb.append('"');
                            break;
                        case '\\':
                            sb.append('\\');
                            break;
                        case '/':
                            sb.append('/');
                            break;
                        case 'n':
                            sb.append('\n');
                            break;
                        case 't':
                            sb.append('\t');
                            break;
                        case 'r':
                            sb.append('\r');
                            break;
                        case 'b':
                            sb.append('\b');
                            break;
                        case 'f':
                            sb.append('\f');
                            break;
                        case 'u':
                            sb.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
                            i += 4;
                            break;
                        default:
                            sb.append(e);
                    }
                } else {
                    sb.append(c);
                }
            }
            return sb.toString();
        }

        private Object num() {
            int st = i;
            while (i < s.length()) {
                char c = s.charAt(i);
                if (c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || (c >= '0' && c <= '9')) {
                    i++;
                } else {
                    break;
                }
            }
            String n = s.substring(st, i);
            try {
                if (n.contains(".") || n.contains("e") || n.contains("E")) {
                    return Double.parseDouble(n);
                }
                return Long.parseLong(n);
            } catch (Exception e) {
                return n;
            }
        }

        private void ws() {
            while (i < s.length()) {
                char c = s.charAt(i);
                if (c == ' ' || c == '\n' || c == '\t' || c == '\r') {
                    i++;
                } else {
                    break;
                }
            }
        }

        private static String quote(String v) {
            StringBuilder sb = new StringBuilder("\"");
            for (int k = 0; k < v.length(); k++) {
                char c = v.charAt(k);
                switch (c) {
                    case '"':
                        sb.append("\\\"");
                        break;
                    case '\\':
                        sb.append("\\\\");
                        break;
                    case '\n':
                        sb.append("\\n");
                        break;
                    case '\r':
                        sb.append("\\r");
                        break;
                    case '\t':
                        sb.append("\\t");
                        break;
                    default:
                        sb.append(c);
                }
            }
            sb.append('"');
            return sb.toString();
        }
    }
}
