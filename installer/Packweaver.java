package packweaver;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.lang.reflect.Method;
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
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
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
    private static final String UA = "packweave-installer/2.0.0 (+https://github.com/packweavers/packweave)";
    private static final String STATE_FILE = ".packweaver-state.json";
    private static final String WORK_DIR = ".packweaver";
    private static final String RELEASE_PREFIX = "release+";
    private static final String CHECKED = "packweaver.checked";
    private static final String RULE = "================================================================";
    private static final Pattern RELEASE = Pattern.compile("\\d+(\\.\\d+)+");
    private static final String[] LOADERS = { "fabric", "quilt", "forge", "neoforge" };
    private static final AtomicBoolean RAN = new AtomicBoolean();
    private static final AtomicBoolean MODS_CHANGED = new AtomicBoolean();
    private static final AtomicBoolean STATE_WARNED = new AtomicBoolean();

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
                finishStage(dir, 600);
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
            if (inGame && System.getProperty(CHECKED) == null) {
                Path dir = findGameDir();
                if (dir != null) {
                    checkVersions(dir, true);
                }
            }
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
        String clean = cleanUrl(url);
        if (clean == null) {
            throw new IOException("Invalid pack URL: " + url.trim());
        }
        url = clean;
        try {
            finishStage(dir, 1);
        } catch (Exception e) {
            System.err.println("  ! couldn't finish the changes queued last time: " + e.getMessage());
        }
        if (Files.exists(dir.resolve(WORK_DIR).resolve("pending.json"))) {
            scheduleHelper(dir);
        }
        Path urlFile = dir.resolve("modpack.url");
        String want = url + "\n";
        if (!Files.exists(urlFile) || !want.equals(new String(Files.readAllBytes(urlFile), StandardCharsets.UTF_8))) {
            Files.write(urlFile, want.getBytes(StandardCharsets.UTF_8));
        }
        Exception failure = null;
        try {
            if (url.startsWith(RELEASE_PREFIX)) {
                runRelease(dir, url.substring(RELEASE_PREFIX.length()), env, inGame);
            } else {
                runCommit(dir, url, env, inGame);
            }
        } catch (Exception e) {
            failure = e;
        }
        checkVersions(dir, inGame);
        if (failure != null) {
            throw failure;
        }
        if (inGame && MODS_CHANGED.get()) {
            System.out.println("packweaver: mods updated, closing the game. start it again to play");
            System.exit(0);
        }
    }

    private static void runCommit(Path dir, String url, String env, boolean inGame) throws Exception {
        String manifestText = "";
        Map<String, Object> mp = null;
        try {
            String raw = remoteText(dir, url + "/modpack.json", "modpack.json");
            if (raw == null) {
                throw new IOException("HTTP 404 for " + url + "/modpack.json");
            }
            manifestText = raw;
            cacheManifest(dir, raw.getBytes(StandardCharsets.UTF_8));
            mp = asMap(Json.parse(raw));
            String mc = str(mp.get("minecraft"));
            String loader = str(mp.get("loader"));
            System.out.println("packweaver: syncing from " + url
                    + (mc != null ? " (Minecraft " + mc + (loader != null ? " / " + loader : "") + ")" : ""));
        } catch (Exception e) {
            System.out.println("packweaver: syncing from " + url);
            System.err.println("  ! couldn't read modpack.json: " + e.getMessage());
        }
        String indexText = remoteText(dir, url + "/index.json", "index.json");
        if (indexText == null) {
            throw new IOException("HTTP 404 for " + url + "/index.json");
        }
        String rulesText;
        try {
            rulesText = remoteText(dir, url + "/.packweaveunlock", ".packweaveunlock");
            if (rulesText == null) {
                rulesText = "";
            }
        } catch (IOException e) {
            rulesText = readRulesText(dir);
            if (rulesText == null) {
                throw e;
            }
        }
        saveRules(dir, rulesText);
        Preserve preserve = parsePreserve(rulesText);
        String signature = "raw:" + sha1Of((manifestText + "\u0000" + indexText + "\u0000" + rulesText
                + "\u0000" + env).getBytes(StandardCharsets.UTF_8));
        if (signature.equals(loadMeta(dir).tag)) {
            List<String> needs = verifyFiles(dir, preserve);
            if (needs.isEmpty()) {
                System.out.println("packweaver: up to date");
                return;
            }
            System.out.println("packweaver: repairing " + needs.size()
                    + (needs.size() == 1 ? " file" : " files"));
            List<Task> tasks = rawTasks(dir, url, indexText, env, mp);
            apply(dir, repairTasks(dir, tasks, needs), Collections.<String, byte[]>emptyMap(), preserve,
                    inGame, null, null, true);
            return;
        }
        apply(dir, rawTasks(dir, url, indexText, env, mp), Collections.<String, byte[]>emptyMap(), preserve,
                inGame, signature, null, false);
    }

    private static List<Task> rawTasks(final Path dir, String url, String indexText, final String env,
            Map<String, Object> mp) throws Exception {
        Map<String, Object> index = asMap(Json.parse(indexText));
        final Map<String, String> contents = new LinkedHashMap<String, String>();
        List<Task> tasks = new ArrayList<Task>();
        for (Object eo : asList(index.get("files"))) {
            Map<String, Object> f = asMap(eo);
            String path = str(f.get("path"));
            if (path == null || path.isEmpty()) {
                continue;
            }
            String sha1 = str(f.get("sha1"));
            if (path.startsWith("content/") && path.endsWith(".json")) {
                contents.put(path, sha1);
            } else if (path.startsWith("overrides/")) {
                String inst = path.substring("overrides/".length());
                if (!inst.isEmpty()) {
                    tasks.add(new Task(inst, url + "/" + enc(path), sha1));
                }
            } else if (isDocument(path)) {
                tasks.add(new Task(path, url + "/" + enc(path), sha1));
            }
        }
        final String base = url;
        final String datapacks = datapackFolder(mp);
        ExecutorService pool = Executors.newFixedThreadPool(8);
        List<Future<Task>> futures = new ArrayList<Future<Task>>();
        for (final Map.Entry<String, String> c : contents.entrySet()) {
            futures.add(pool.submit(new java.util.concurrent.Callable<Task>() {
                public Task call() throws Exception {
                    String text = contentJson(dir, base, c.getKey(), c.getValue());
                    return contentTask(c.getKey(), asMap(Json.parse(text)), env, datapacks);
                }
            }));
        }
        try {
            for (Future<Task> ft : futures) {
                Task t;
                try {
                    t = ft.get();
                } catch (java.util.concurrent.ExecutionException e) {
                    Throwable cause = e.getCause() == null ? e : e.getCause();
                    throw new IOException(cause.getMessage(), cause);
                }
                if (t != null) {
                    tasks.add(t);
                }
            }
        } finally {
            pool.shutdownNow();
        }
        pruneContent(dir, contents);
        return tasks;
    }

    private static String contentJson(Path dir, String url, String path, String sha1) throws Exception {
        Path cached = sha1 == null || sha1.isEmpty() ? null
                : dir.resolve(WORK_DIR).resolve("content")
                        .resolve(sha1.toLowerCase(java.util.Locale.ROOT) + ".json");
        if (cached != null && Files.isRegularFile(cached)) {
            return new String(Files.readAllBytes(cached), StandardCharsets.UTF_8);
        }
        byte[] bytes = getBytes(url + "/" + enc(path));
        if (cached != null) {
            if (!sha1.equalsIgnoreCase(sha1Of(bytes))) {
                throw new IOException(path + " doesn't match index.json yet. Try again in a few minutes");
            }
            Files.createDirectories(cached.getParent());
            atomicWrite(cached, bytes);
        }
        return new String(bytes, StandardCharsets.UTF_8);
    }

    private static void pruneContent(Path dir, Map<String, String> contents) {
        Set<String> keep = new HashSet<String>();
        for (String sha1 : contents.values()) {
            if (sha1 != null) {
                keep.add(sha1.toLowerCase(java.util.Locale.ROOT) + ".json");
            }
        }
        File[] cached = dir.resolve(WORK_DIR).resolve("content").toFile().listFiles();
        if (cached == null) {
            return;
        }
        for (File f : cached) {
            if (!keep.contains(f.getName())) {
                f.delete();
            }
        }
    }

    private static final class Remote {
        private int code;
        private byte[] body;
        private String etag;
    }

    private static Remote fetchRemote(String url, String etag) throws IOException {
        String current = url;
        for (int i = 0; i < 5; i++) {
            HttpURLConnection c = (HttpURLConnection) new URL(current).openConnection();
            c.setInstanceFollowRedirects(false);
            c.setConnectTimeout(15000);
            c.setReadTimeout(60000);
            c.setRequestProperty("User-Agent", UA);
            if (etag != null && !etag.isEmpty()) {
                c.setRequestProperty("If-None-Match", etag);
            }
            Remote r = new Remote();
            r.code = c.getResponseCode();
            if (r.code == 304 || r.code == 404) {
                c.disconnect();
                return r;
            }
            if (r.code / 100 == 3) {
                String loc = c.getHeaderField("Location");
                c.disconnect();
                if (loc == null) {
                    throw new IOException("HTTP " + r.code + " for " + current);
                }
                current = new URL(new URL(current), loc).toString();
                continue;
            }
            if (r.code / 100 != 2) {
                c.disconnect();
                throw new IOException("HTTP " + r.code + " for " + current);
            }
            r.etag = c.getHeaderField("ETag");
            InputStream in = c.getInputStream();
            try {
                r.body = readAll(in);
            } finally {
                in.close();
            }
            return r;
        }
        throw new IOException("Too many redirects for " + url);
    }

    private static String remoteText(Path dir, String url, String name) throws IOException {
        Path folder = dir.resolve(WORK_DIR).resolve("remote");
        Path body = folder.resolve(name);
        Path tagFile = folder.resolve(name + ".etag");
        String etag = null;
        if (Files.isRegularFile(body) && Files.isRegularFile(tagFile)) {
            etag = new String(Files.readAllBytes(tagFile), StandardCharsets.UTF_8).trim();
        }
        Remote r = fetchRemote(url, etag);
        if (r.code == 304 && Files.isRegularFile(body)) {
            return new String(Files.readAllBytes(body), StandardCharsets.UTF_8);
        }
        if (r.code == 304) {
            r = fetchRemote(url, null);
        }
        if (r.code == 404) {
            Files.deleteIfExists(body);
            Files.deleteIfExists(tagFile);
            return null;
        }
        Files.createDirectories(folder);
        atomicWrite(body, r.body);
        if (r.etag != null && !r.etag.isEmpty()) {
            atomicWrite(tagFile, r.etag.getBytes(StandardCharsets.UTF_8));
        } else {
            Files.deleteIfExists(tagFile);
        }
        return new String(r.body, StandardCharsets.UTF_8);
    }

    private static void runRelease(Path dir, String webUrl, String env, boolean inGame) throws Exception {
        System.out.println("packweaver: syncing from " + webUrl);
        String token = readToken(dir);
        String repo = ownerRepo(webUrl);
        String repoApi = apiBase(webUrl) + "/repos/" + repo;
        Meta meta = loadMeta(dir);
        String lastTag = meta.tag;
        boolean needManifest = !Files.isRegularFile(dir.resolve(WORK_DIR).resolve("modpack.json"));
        ApiResp resp = apiGet(repoApi + "/releases/latest", token,
                lastTag != null && !needManifest ? meta.etag : null);
        if (resp.code == 304) {
            verifyLocal(dir, repoApi, token, env, inGame, lastTag);
            return;
        }
        if (resp.code == 404) {
            throw new IOException("No releases found for " + repo + ".");
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
        if (tag.equals(lastTag) && !needManifest) {
            saveState(dir, loadState(dir), tag, resp.etag, null);
            verifyLocal(dir, repoApi, token, env, inGame, tag);
            return;
        }
        System.out.println("packweaver: updating to " + tag);
        String packUrl = assets.get("pack.zip");
        String deltaName = lastTag == null || needManifest ? null : "delta-" + lastTag + ".zip";
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
        byte[] mpBytes = entries.get("modpack.json");
        if (mpBytes != null) {
            cacheManifest(dir, mpBytes);
        }
        byte[] newToken = entries.get("modpack.token");
        if (newToken != null) {
            String t = cleanToken(new String(newToken, StandardCharsets.UTF_8));
            if (t != null && !t.equals(token)) {
                Files.write(dir.resolve("modpack.token"), (t + "\n").getBytes(StandardCharsets.UTF_8));
            }
        }
        byte[] newUrl = entries.get("modpack.url");
        if (newUrl != null) {
            String moved = cleanUrl(new String(newUrl, StandardCharsets.UTF_8));
            if (moved != null && !moved.equalsIgnoreCase(RELEASE_PREFIX + webUrl)) {
                Files.write(dir.resolve("modpack.url"), (moved + "\n").getBytes(StandardCharsets.UTF_8));
                System.out.println("packweaver: the pack moved to " + moved);
            }
        }
        saveRules(dir, in.rules);
        apply(dir, in.tasks, in.local, in.preserve, inGame, tag, resp.etag, false);
    }

    private static void verifyLocal(Path dir, String repoApi, String token, String env,
            boolean inGame, String tag) throws Exception {
        Preserve rules = readRules(dir);
        SyncInput in = null;
        if (rules == null) {
            in = fetchRelease(repoApi, token, env);
            saveRules(dir, in.rules);
            rules = in.preserve;
        }
        List<String> needs = verifyFiles(dir, rules);
        if (needs.isEmpty()) {
            System.out.println("packweaver: up to date (" + tag + ")");
            return;
        }
        System.out.println("packweaver: repairing " + needs.size()
                + (needs.size() == 1 ? " file" : " files"));
        if (in == null) {
            in = fetchRelease(repoApi, token, env);
            saveRules(dir, in.rules);
        }
        apply(dir, repairTasks(dir, in.tasks, needs), in.local, in.preserve, inGame, null, null, true);
    }

    private static SyncInput fetchRelease(String repoApi, String token, String env) throws Exception {
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
        return buildFromZip(unzip(fetchBytes(packUrl, token, "application/octet-stream")), env);
    }

    private static List<Task> repairTasks(Path dir, List<Task> all, List<String> needs) {
        String copy = jarCopy(dir, ownJarRel());
        boolean copyNeeded = copy != null && needs.contains(copy);
        List<Task> tasks = new ArrayList<Task>();
        for (Task t : all) {
            if (needs.contains(t.rel) || (copyNeeded && t.rel.equals("packweaver.jar"))) {
                tasks.add(t);
            }
        }
        return tasks;
    }

    private static List<String> verifyFiles(Path dir, Preserve rules) throws Exception {
        Map<String, String> state = loadState(dir);
        Map<String, String> stats = loadStats(dir);
        List<String> needs = new ArrayList<String>();
        boolean changed = false;
        for (Map.Entry<String, String> e : state.entrySet()) {
            String rel = e.getKey();
            Path p = dir.resolve(rel);
            if (!Files.isRegularFile(p)) {
                if (!Files.isRegularFile(dir.resolve(rel + ".disabled"))) {
                    needs.add(rel);
                }
                continue;
            }
            String before = stats.get(rel);
            String disk = cachedSha(stats, state, dir, rel);
            if (!String.valueOf(before).equals(String.valueOf(stats.get(rel)))) {
                changed = true;
            }
            if (disk == null) {
                needs.add(rel);
                continue;
            }
            if (disk.equalsIgnoreCase(e.getValue())) {
                continue;
            }
            boolean unlocked = rules.matches(rel);
            List<String> lockedKeys = rules.keys(rel, false);
            if (unlocked && lockedKeys == null) {
                continue;
            }
            byte[] base = readBaseBytes(dir, rel);
            if (base == null) {
                needs.add(rel);
                continue;
            }
            byte[] current = Files.readAllBytes(p);
            byte[] fixed;
            if (unlocked) {
                fixed = forceSettings(rel, base, current, lockedKeys);
            } else {
                List<String> keys = rules.keys(rel, true);
                byte[] merged = keys == null ? null : mergeSettings(dir, rel, base, current, keys);
                fixed = merged != null ? merged : base;
                if (sameSettings(rel, fixed, current)) {
                    e.setValue(disk.toLowerCase(java.util.Locale.ROOT));
                    changed = true;
                    continue;
                }
            }
            if (fixed == null || Arrays.equals(fixed, current)) {
                continue;
            }
            Files.write(p, fixed);
            String sha = sha1Of(fixed);
            recordStat(stats, dir, rel, sha);
            if (!unlocked) {
                e.setValue(sha);
            }
            changed = true;
            System.out.println("  ~ " + (unlocked ? "reset locked settings in " : "restored ") + rel);
        }
        if (changed) {
            saveState(dir, state, null, null, stats);
        }
        return needs;
    }

    private static boolean sameSettings(String rel, byte[] want, byte[] have) {
        String a = new String(want, StandardCharsets.UTF_8);
        String b = new String(have, StandardCharsets.UTF_8);
        Map<String, int[]> wanted = settingSpans(rel, a);
        Map<String, int[]> found = settingSpans(rel, b);
        if (wanted == null || found == null || wanted.isEmpty()) {
            return false;
        }
        for (Map.Entry<String, int[]> e : wanted.entrySet()) {
            int[] h = found.get(e.getKey());
            if (h == null || !a.substring(e.getValue()[0], e.getValue()[1]).equals(b.substring(h[0], h[1]))) {
                return false;
            }
        }
        return true;
    }

    private static final class SyncInput {
        private final List<Task> tasks = new ArrayList<Task>();
        private final Map<String, byte[]> local = new HashMap<String, byte[]>();
        private Preserve preserve = new Preserve();
        private String rules = "";
    }

    private static final class Task {
        private final String rel;
        private final String url;
        private final String sha1;

        private Task(String rel, String url, String sha1) {
            this.rel = rel;
            this.url = url;
            this.sha1 = sha1;
        }
    }

    private static final class Pending {
        private final String op;
        private final String rel;
        private final String sha1;
        private final String staged;

        private Pending(String op, String rel, String sha1, String staged) {
            this.op = op;
            this.rel = rel;
            this.sha1 = sha1;
            this.staged = staged;
        }
    }

    private static final class Versions {
        private final String minecraft;
        private final String loader;
        private final String loaderVersion;

        private Versions(String minecraft, String loader, String loaderVersion) {
            this.minecraft = minecraft;
            this.loader = loader;
            this.loaderVersion = loaderVersion;
        }
    }

    private static final class Meta {
        private final String tag;
        private final String etag;

        private Meta(String tag, String etag) {
            this.tag = tag;
            this.etag = etag;
        }
    }

    private static SyncInput buildFromZip(Map<String, byte[]> entries, String env) throws IOException {
        SyncInput in = new SyncInput();
        Map<String, Object> manifest = null;
        byte[] mpBytes = entries.get("modpack.json");
        if (mpBytes != null) {
            try {
                manifest = asMap(Json.parse(new String(mpBytes, StandardCharsets.UTF_8)));
            } catch (Exception e) {
                System.err.println("  ! the release's modpack.json isn't valid: " + e.getMessage());
            }
        }
        String datapacks = datapackFolder(manifest);
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
                Task t = contentTask(path,
                        asMap(Json.parse(new String(cj, StandardCharsets.UTF_8))), env, datapacks);
                if (t != null) {
                    in.tasks.add(t);
                }
            } else if (path.startsWith("overrides/")) {
                String inst = path.substring("overrides/".length());
                if (inst.isEmpty()) {
                    continue;
                }
                in.tasks.add(new Task(inst, null, str(f.get("sha1"))));
                byte[] bytes = entries.get(path);
                if (bytes != null) {
                    in.local.put(inst, bytes);
                }
            } else if (isDocument(path)) {
                in.tasks.add(new Task(path, null, str(f.get("sha1"))));
                byte[] bytes = entries.get(path);
                if (bytes != null) {
                    in.local.put(path, bytes);
                }
            }
        }
        byte[] pres = entries.get(".packweaveunlock");
        if (pres != null) {
            in.rules = new String(pres, StandardCharsets.UTF_8);
            in.preserve = parsePreserve(in.rules);
        }
        return in;
    }

    private static boolean needsFull(Path dir, SyncInput in) {
        for (Task t : in.tasks) {
            if (t.url != null || in.local.containsKey(t.rel)) {
                continue;
            }
            String want = t.sha1;
            String disk = diskSha(dir.resolve(t.rel));
            if (want == null || disk == null || !want.equalsIgnoreCase(disk)) {
                return true;
            }
        }
        return false;
    }

    private static boolean isDocument(String path) {
        return path.indexOf('/') < 0 && path.toLowerCase(java.util.Locale.ROOT).endsWith(".md");
    }

    private static String datapackFolder(Map<String, Object> manifest) {
        String folder = manifest == null ? null : safeRel(str(manifest.get("datapacks")));
        return folder == null ? "datapacks" : folder;
    }

    private static Task contentTask(String cp, Map<String, Object> m, String env, String datapacks) {
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
        String folder = "shaders".equals(sub) ? "shaderpacks" : "datapacks".equals(sub) ? datapacks : sub;
        return new Task(folder + "/" + fn, dl, str(src.get("sha1")));
    }

    private static int apply(Path dir, List<Task> tasksIn, final Map<String, byte[]> local,
            final Preserve preserve, boolean inGame, String tag, String etag, boolean partial)
            throws Exception {
        LinkedHashMap<String, String> desired = new LinkedHashMap<String, String>();
        List<Task> downloads = new ArrayList<Task>();
        for (Task t : tasksIn) {
            String rel = safeRel(t.rel);
            if (rel == null) {
                System.err.println("  ! skipped unsafe path: " + t.rel);
                continue;
            }
            if (protectedPath(rel) || desired.containsKey(rel)) {
                continue;
            }
            desired.put(rel, t.sha1);
            downloads.add(new Task(rel, t.url, t.sha1));
        }
        final String ownRel = ownJarRel();
        String copy = jarCopy(dir, ownRel);
        Task packJar = null;
        for (Task t : downloads) {
            if (t.rel.equals("packweaver.jar")) {
                packJar = t;
            }
        }
        if (packJar != null && copy != null && !desired.containsKey(copy)) {
            desired.put(copy, packJar.sha1);
            downloads.add(new Task(copy, packJar.url, packJar.sha1));
            byte[] jarBytes = local.get("packweaver.jar");
            if (jarBytes != null) {
                local.put(copy, jarBytes);
            }
        }

        Map<String, String> state = loadState(dir);
        final Map<String, String> newState = partial
                ? new ConcurrentHashMap<String, String>(state)
                : new ConcurrentHashMap<String, String>();
        final Map<String, String> newStats =
                new ConcurrentHashMap<String, String>(loadStats(dir));
        final List<Pending> pending = Collections.synchronizedList(new ArrayList<Pending>());
        final AtomicInteger modsChanged = new AtomicInteger();

        for (Map.Entry<String, String> e : partial
                ? Collections.<String, String>emptyMap().entrySet()
                : state.entrySet()) {
            String old = e.getKey();
            if (desired.containsKey(old)) {
                continue;
            }
            if (protectedPath(old) || ownJarPath(old) || old.equals(ownRel) || safeRel(old) == null) {
                continue;
            }
            Path p = dir.resolve(old);
            String disk = cachedSha(newStats, state, dir, old);
            if (disk == null) {
                try {
                    Files.deleteIfExists(dir.resolve(old + ".disabled"));
                } catch (Exception ex) {
                    System.err.println("  ! couldn't remove " + old + ".disabled: " + ex.getMessage());
                }
            }
            if (disk != null && preserve.matches(old) && !disk.equalsIgnoreCase(e.getValue())) {
                System.out.println("  ~ kept " + old + " (edited locally)");
                continue;
            }
            if (disk != null && old.startsWith("mods/")) {
                modsChanged.incrementAndGet();
            }
            if (disk != null && inGame && old.startsWith("mods/")) {
                pending.add(new Pending("del", old, null, null));
                newState.put(old, e.getValue());
                continue;
            }
            try {
                Files.deleteIfExists(p);
                newStats.remove(old);
            } catch (Exception ex) {
                pending.add(new Pending("del", old, null, null));
                newState.put(old, e.getValue());
            }
        }

        final Path root = dir;
        final boolean game = inGame;
        final Map<String, String> prevState = state;
        ExecutorService pool = Executors.newFixedThreadPool(8);
        List<Future<?>> futures = new ArrayList<Future<?>>();
        final int total = downloads.size();
        final AtomicInteger done = new AtomicInteger();
        final AtomicInteger failed = new AtomicInteger();
        final AtomicInteger kept = new AtomicInteger();
        for (final Task d : downloads) {
            futures.add(pool.submit(new Runnable() {
                public void run() {
                    final String rel = d.rel;
                    final String src = d.url;
                    final String sha1 = d.sha1;
                    try {
                        Path out = root.resolve(rel);
                        String disk = cachedSha(newStats, prevState, root, rel);
                        String wrote = prevState.get(rel);
                        boolean unlocked = preserve.matches(rel);
                        List<String> keys = preserve.keys(rel, true);
                        List<String> lockedKeys = preserve.keys(rel, false);
                        boolean edited = disk != null && unlocked
                                && (wrote == null || !wrote.equalsIgnoreCase(disk));
                        if (sha1 != null && !sha1.isEmpty() && disk != null && sha1.equalsIgnoreCase(disk)) {
                            if (keys != null || lockedKeys != null || partial) {
                                saveBase(root, rel, Files.readAllBytes(out));
                            }
                            newState.put(rel, disk.toLowerCase());
                            recordStat(newStats, root, rel, disk);
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
                        if (edited && lockedKeys == null) {
                            if (wrote != null) {
                                newState.put(rel, wrote);
                            }
                            kept.incrementAndGet();
                            System.out.println("  ~ kept " + rel + " (edited locally)");
                            return;
                        }
                        byte[] pre = local.get(rel);
                        byte[] saved = pre == null ? readBaseBytes(root, rel) : null;
                        byte[] bytes;
                        if (pre != null) {
                            bytes = pre;
                        } else if (saved != null && sha1 != null && sha1.equalsIgnoreCase(sha1Of(saved))) {
                            bytes = saved;
                        } else if (src != null) {
                            bytes = getBytes(src);
                        } else {
                            throw new IOException("no data in release");
                        }
                        String got = sha1Of(bytes);
                        if (sha1 != null && !sha1.isEmpty() && !sha1.equalsIgnoreCase(got)) {
                            throw new IOException("checksum mismatch");
                        }
                        if (edited) {
                            saveBase(root, rel, bytes);
                            byte[] forced = forceSettings(rel, bytes, Files.readAllBytes(out), lockedKeys);
                            if (wrote != null) {
                                newState.put(rel, wrote);
                            }
                            kept.incrementAndGet();
                            if (forced == null) {
                                System.out.println("  ~ kept " + rel + " (edited locally)");
                                return;
                            }
                            Files.write(out, forced);
                            recordStat(newStats, root, rel, sha1Of(forced));
                            System.out.println("  ~ kept " + rel + " (edited locally, locked settings updated)");
                            return;
                        }
                        if (keys != null && disk != null) {
                            byte[] merged = mergeSettings(root, rel, bytes, Files.readAllBytes(out), keys);
                            saveBase(root, rel, bytes);
                            if (merged != null) {
                                String mergedSha = sha1Of(merged);
                                if (mergedSha.equalsIgnoreCase(disk)) {
                                    newState.put(rel, disk.toLowerCase());
                                    recordStat(newStats, root, rel, disk);
                                    kept.incrementAndGet();
                                    done.incrementAndGet();
                                    return;
                                }
                                bytes = merged;
                                got = mergedSha;
                                System.out.println("  ~ kept your settings in " + rel);
                            }
                        } else if (keys != null || lockedKeys != null || partial) {
                            saveBase(root, rel, bytes);
                        }
                        if (rel.startsWith("mods/") && !rel.equals(ownRel)) {
                            modsChanged.incrementAndGet();
                        }
                        boolean stage = (game && rel.startsWith("mods/") && disk != null)
                                || rel.equals(ownRel);
                        if (!stage) {
                            try {
                                Files.createDirectories(out.getParent());
                                Files.write(out, bytes);
                                newState.put(rel, got);
                                recordStat(newStats, root, rel, got);
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
                        pending.add(new Pending("put", rel, got, root.relativize(sf).toString().replace('\\', '/')));
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
        if (modsChanged.get() > 0) {
            MODS_CHANGED.set(true);
        }
        return failed.get();
    }

    private static String statOf(Path p) {
        try {
            return Files.size(p) + "," + Files.getLastModifiedTime(p).toMillis();
        } catch (Exception e) {
            return null;
        }
    }

    private static void recordStat(Map<String, String> stats, Path root, String rel, String sha) {
        String cur = statOf(root.resolve(rel));
        if (cur != null && sha != null) {
            stats.put(rel, cur + "," + sha.toLowerCase(java.util.Locale.ROOT));
        }
    }

    private static String cachedSha(Map<String, String> stats, Map<String, String> state, Path root, String rel) {
        Path p = root.resolve(rel);
        String entry = stats.get(rel);
        String cur = statOf(p);
        if (entry != null && cur != null) {
            if (entry.equals(cur) && state.get(rel) != null) {
                return state.get(rel);
            }
            if (entry.startsWith(cur + ",")) {
                return entry.substring(cur.length() + 1);
            }
        }
        String disk = diskSha(p);
        if (disk != null) {
            recordStat(stats, root, rel, disk);
        }
        return disk;
    }

    private static void saveRules(Path dir, String text) {
        try {
            Path f = dir.resolve(WORK_DIR).resolve("unlock");
            Files.createDirectories(f.getParent());
            atomicWrite(f, (text == null ? "" : text).getBytes(StandardCharsets.UTF_8));
        } catch (Exception e) {
            System.err.println("  ! couldn't save the unlock rules: " + e.getMessage());
        }
    }

    private static String readRulesText(Path dir) {
        try {
            Path f = dir.resolve(WORK_DIR).resolve("unlock");
            return Files.isRegularFile(f) ? new String(Files.readAllBytes(f), StandardCharsets.UTF_8) : null;
        } catch (Exception e) {
            System.err.println("  ! couldn't read the saved unlock rules: " + e.getMessage());
            return null;
        }
    }

    private static Preserve readRules(Path dir) {
        String text = readRulesText(dir);
        return text == null ? null : parsePreserve(text);
    }

    private static String cleanUrl(String raw) {
        String url = raw.replace("\uFEFF", "").replace("\"", "").replace("'", "").trim();
        if (url.startsWith(RELEASE_PREFIX)) {
            url = RELEASE_PREFIX + url.substring(RELEASE_PREFIX.length()).trim();
        }
        for (int i = 0; i < url.length(); i++) {
            if (Character.isWhitespace(url.charAt(i))) {
                url = url.substring(0, i);
                break;
            }
        }
        url = url.replaceAll("/+$", "");
        if (!url.startsWith("http://") && !url.startsWith("https://") && !url.startsWith(RELEASE_PREFIX)) {
            return null;
        }
        return url;
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
        } catch (Exception e) {
            System.err.println("  ! couldn't read modpack.token: " + e.getMessage());
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

    private static String ownerRepo(String webUrl) throws IOException {
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
        return owner + "/" + repo;
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
        if (r.code == 401 && token != null && !token.isEmpty()) {
            c.disconnect();
            ApiResp open = apiGet(url, null, etag);
            if (open.code / 100 == 2 || open.code == 304) {
                return open;
            }
            throw new IOException("HTTP 401 for " + url);
        }
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

    private static String loaderUid(String loader) {
        if (loader == null) {
            return null;
        }
        String l = loader.trim().toLowerCase();
        if (l.equals("fabric")) {
            return "net.fabricmc.fabric-loader";
        }
        if (l.equals("quilt")) {
            return "org.quiltmc.quilt-loader";
        }
        if (l.equals("forge")) {
            return "net.minecraftforge";
        }
        if (l.equals("neoforge") || l.equals("neoforged")) {
            return "net.neoforged";
        }
        return null;
    }

    private static Path instanceRoot(Path gameDir) {
        Path parent = gameDir.getParent();
        if (parent != null && Files.isRegularFile(parent.resolve("mmc-pack.json"))) {
            return parent;
        }
        if (Files.isRegularFile(gameDir.resolve("mmc-pack.json"))) {
            return gameDir;
        }
        return null;
    }

    private static String componentVersion(List<Object> comps, String uid) {
        for (Object o : comps) {
            Map<String, Object> c = asMap(o);
            if (uid.equals(str(c.get("uid")))) {
                String v = str(c.get("version"));
                return v == null ? "" : v;
            }
        }
        return null;
    }

    private static void cacheManifest(Path dir, byte[] bytes) {
        try {
            Path f = dir.resolve(WORK_DIR).resolve("modpack.json");
            Files.createDirectories(f.getParent());
            atomicWrite(f, bytes);
        } catch (Exception e) {
            System.err.println("  ! couldn't save modpack.json: " + e.getMessage());
        }
    }

    private static Map<String, Object> cachedManifest(Path dir) {
        Path f = dir.resolve(WORK_DIR).resolve("modpack.json");
        if (!Files.isRegularFile(f)) {
            return null;
        }
        try {
            return asMap(Json.parse(new String(Files.readAllBytes(f), StandardCharsets.UTF_8)));
        } catch (Exception e) {
            System.err.println("  ! could not read modpack.json: " + e.getMessage());
            return null;
        }
    }

    private static void checkVersions(Path dir, boolean inGame) {
        Map<String, Object> mp = cachedManifest(dir);
        if (mp == null) {
            return;
        }
        Versions have = inGame ? runningVersions() : instanceVersions(dir);
        if (have == null) {
            return;
        }
        System.setProperty(CHECKED, "1");
        String wantMc = blank(str(mp.get("minecraft")));
        String wantLoader = blank(str(mp.get("loader")));
        String wantLv = blank(str(mp.get("loaderVersion")));
        boolean stop = false;
        if (wantMc != null && have.minecraft != null && !wantMc.equals(have.minecraft)) {
            stop = RELEASE.matcher(wantMc).matches() && RELEASE.matcher(have.minecraft).matches();
            banner("WRONG MINECRAFT VERSION",
                    "This pack needs Minecraft " + wantMc + ". This instance runs Minecraft " + have.minecraft + ".",
                    "Change the instance's Minecraft version to " + wantMc + " in your launcher"
                            + (stop ? ", then start the game again." : ". The game may crash until you do."));
        }
        if (wantLoader != null && !wantLoader.equalsIgnoreCase("vanilla")) {
            String loader = wantLoader.toLowerCase();
            String wanted = loaderText(loader, wantLv);
            if (!sameLoader(loader, have.loader, wantMc)) {
                banner("WRONG MOD LOADER",
                        "This pack uses " + wanted + ". This instance runs "
                                + (have.loader == null ? "no mod loader" : loaderText(have.loader, have.loaderVersion))
                                + ".",
                        "Switch the instance to " + wanted + " in your launcher. The game may crash until you do.");
            } else if (wantLv != null && have.loaderVersion != null && !wantLv.equals(have.loaderVersion)) {
                banner("WRONG LOADER VERSION",
                        "This pack uses " + wanted + ". This instance runs " + loaderText(loader, have.loaderVersion)
                                + ".",
                        "Change the instance's " + loaderName(loader) + " version to " + wantLv
                                + " in your launcher. The game may crash until you do.");
            }
        }
        if (stop) {
            System.err.println(inGame ? "packweaver: closing the game" : "packweaver: stopping this launch");
            System.exit(1);
        }
    }

    private static void banner(String title, String problem, String fix) {
        System.err.println("packweaver: " + RULE);
        System.err.println("packweaver: " + title);
        System.err.println("packweaver: " + problem);
        System.err.println("packweaver: " + fix);
        System.err.println("packweaver: " + RULE);
    }

    private static boolean sameLoader(String want, String have, String minecraft) {
        if (want.equals(have)) {
            return true;
        }
        return want.equals("neoforge") && "forge".equals(have) && "1.20.1".equals(minecraft);
    }

    private static String loaderText(String loader, String version) {
        return version == null ? loaderName(loader) : loaderName(loader) + " " + version;
    }

    private static String loaderName(String loader) {
        if (loader.equals("fabric")) {
            return "Fabric";
        }
        if (loader.equals("quilt")) {
            return "Quilt";
        }
        if (loader.equals("forge")) {
            return "Forge";
        }
        if (loader.equals("neoforge")) {
            return "NeoForge";
        }
        return loader;
    }

    private static Versions instanceVersions(Path dir) {
        Path root = instanceRoot(dir);
        if (root == null) {
            return null;
        }
        try {
            Map<String, Object> pack = asMap(Json.parse(new String(
                    Files.readAllBytes(root.resolve("mmc-pack.json")), StandardCharsets.UTF_8)));
            List<Object> comps = asList(pack.get("components"));
            String mc = blank(componentVersion(comps, "net.minecraft"));
            if (mc == null) {
                return null;
            }
            for (String loader : LOADERS) {
                String version = componentVersion(comps, loaderUid(loader));
                if (version != null) {
                    return new Versions(mc, loader, blank(version));
                }
            }
            return new Versions(mc, null, null);
        } catch (Exception e) {
            return null;
        }
    }

    private static Versions runningVersions() {
        try {
            Class<?> fl = Class.forName("net.fabricmc.loader.api.FabricLoader");
            Object loader = fl.getMethod("getInstance").invoke(null);
            boolean quilt = hasClass("org.quiltmc.loader.api.QuiltLoader");
            return new Versions(fabricVersion(fl, loader, "minecraft"), quilt ? "quilt" : "fabric",
                    fabricVersion(fl, loader, quilt ? "quilt_loader" : "fabricloader"));
        } catch (Throwable ignore) {
        }
        Versions neo = fmlVersions("net.neoforged.fml", "neoforge");
        return neo != null ? neo : fmlVersions("net.minecraftforge.fml", "forge");
    }

    private static Versions fmlVersions(String fml, String loader) {
        try {
            return new Versions(fmlVersion(fml, "minecraft"), loader, fmlVersion(fml, loader));
        } catch (Throwable ignore) {
            return null;
        }
    }

    private static String fabricVersion(Class<?> fl, Object loader, String id) throws Exception {
        Object found = ((Optional<?>) fl.getMethod("getModContainer", String.class).invoke(loader, id)).orElse(null);
        if (found == null) {
            return null;
        }
        Object meta = Class.forName("net.fabricmc.loader.api.ModContainer").getMethod("getMetadata").invoke(found);
        Object version = Class.forName("net.fabricmc.loader.api.metadata.ModMetadata").getMethod("getVersion")
                .invoke(meta);
        Object text = Class.forName("net.fabricmc.loader.api.Version").getMethod("getFriendlyString")
                .invoke(version);
        return text == null ? null : blank(text.toString());
    }

    private static String fmlVersion(String fml, String id) throws Exception {
        Class<?> list = Class.forName(fml + ".ModList");
        Object mods = list.getMethod("get").invoke(null);
        Object found = ((Optional<?>) list.getMethod("getModContainerById", String.class).invoke(mods, id))
                .orElse(null);
        if (found == null) {
            return null;
        }
        Method info = Class.forName(fml + ".ModContainer").getMethod("getModInfo");
        Object version = info.getReturnType().getMethod("getVersion").invoke(info.invoke(found));
        return version == null ? null : blank(version.toString());
    }

    private static boolean hasClass(String name) {
        try {
            Class.forName(name, false, Packweaver.class.getClassLoader());
            return true;
        } catch (Throwable t) {
            return false;
        }
    }

    private static String blank(String s) {
        if (s == null) {
            return null;
        }
        String t = s.trim();
        return t.isEmpty() ? null : t;
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

    private static String jarCopy(Path dir, String ownRel) {
        if (ownRel.startsWith("mods/")) {
            return ownRel;
        }
        return Files.isRegularFile(dir.resolve("mods").resolve("packweaver.jar")) ? "mods/packweaver.jar" : null;
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

    private static void savePending(Path dir, List<Pending> ops) throws IOException {
        StringBuilder sb = new StringBuilder("{\"ops\":[");
        boolean first = true;
        for (Pending op : ops) {
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append("{\"op\":").append(Json.quote(op.op));
            sb.append(",\"path\":").append(Json.quote(op.rel));
            if (op.sha1 != null) {
                sb.append(",\"sha\":").append(Json.quote(op.sha1));
            }
            if (op.staged != null) {
                sb.append(",\"stage\":").append(Json.quote(op.staged));
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
        List<Pending> ops = new ArrayList<Pending>();
        Map<String, Object> parsed = asMap(Json.parse(new String(Files.readAllBytes(pf), StandardCharsets.UTF_8)));
        for (Object oo : asList(parsed.get("ops"))) {
            Map<String, Object> o = asMap(oo);
            ops.add(new Pending(str(o.get("op")), str(o.get("path")), str(o.get("sha")), str(o.get("stage"))));
        }
        Map<String, String> state = loadState(dir);
        for (int r = 0; r < rounds && !ops.isEmpty(); r++) {
            if (r > 0) {
                Thread.sleep(300);
            }
            List<Pending> remaining = new ArrayList<Pending>();
            for (Pending op : ops) {
                try {
                    if (safeRel(op.rel) == null) {
                        state.remove(op.rel);
                        continue;
                    }
                    Path target = dir.resolve(op.rel);
                    if ("del".equals(op.op)) {
                        Files.deleteIfExists(target);
                        state.remove(op.rel);
                    } else {
                        Path sf = op.staged == null || safeRel(op.staged) == null
                                ? null
                                : dir.resolve(op.staged);
                        if (sf == null || !Files.exists(sf)) {
                            continue;
                        }
                        Files.createDirectories(target.getParent());
                        Files.move(sf, target, StandardCopyOption.REPLACE_EXISTING);
                        if (op.sha1 != null) {
                            state.put(op.rel, op.sha1);
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
        if (System.getProperty("packweaver.helper") != null) {
            return;
        }
        System.setProperty("packweaver.helper", "1");
        Runtime.getRuntime().addShutdownHook(new Thread(new Runnable() {
            public void run() {
                try {
                    Path jar = ownJar();
                    if (jar == null) {
                        return;
                    }
                    jar = runnerCopy(dir, jar);
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
                } catch (Exception e) {
                    System.err.println("  ! couldn't start the helper for queued changes: " + e.getMessage());
                }
            }
        }));
    }

    private static Path runnerCopy(Path dir, Path jar) {
        Path work = dir.resolve(WORK_DIR);
        try {
            Files.createDirectories(work);
            for (Path p : listFiles(work)) {
                String name = p.getFileName().toString();
                if (name.startsWith("finish-") && name.endsWith(".jar")) {
                    try {
                        Files.deleteIfExists(p);
                    } catch (Exception ignore) {
                    }
                }
            }
            Path copy = work.resolve("finish-" + System.currentTimeMillis() + ".jar");
            Files.copy(jar, copy, StandardCopyOption.REPLACE_EXISTING);
            return copy;
        } catch (Exception e) {
            return jar;
        }
    }

    private static final class Rule {
        private final boolean unlock;
        private final Pattern path;
        private final String key;

        private Rule(boolean unlock, Pattern path, String key) {
            this.unlock = unlock;
            this.path = path;
            this.key = key;
        }
    }

    private static final class Preserve {
        private final List<Rule> rules = new ArrayList<Rule>();

        private boolean matches(String rel) {
            boolean state = false;
            for (Rule r : rules) {
                if (r.key == null && r.path.matcher(rel).matches()) {
                    state = r.unlock;
                }
            }
            return state;
        }

        private List<String> keys(String rel, boolean unlocked) {
            if (matches(rel) == unlocked) {
                return null;
            }
            Map<String, Boolean> state = new LinkedHashMap<String, Boolean>();
            for (Rule r : rules) {
                if (r.key != null && r.path.matcher(rel).matches()) {
                    state.put(r.key, r.unlock);
                }
            }
            List<String> out = new ArrayList<String>();
            for (Map.Entry<String, Boolean> e : state.entrySet()) {
                if (e.getValue() == unlocked) {
                    out.add(e.getKey());
                }
            }
            return out.isEmpty() ? null : out;
        }
    }

    private static Preserve parsePreserve(String text) {
        Preserve out = new Preserve();
        for (String line : text.split("\n")) {
            Rule rule = preserveRule(line);
            if (rule != null) {
                out.rules.add(rule);
            }
        }
        return out;
    }

    private static Rule preserveRule(String line) {
        String t = line.replace("\uFEFF", "").trim();
        if (t.isEmpty() || t.startsWith("#")) {
            return null;
        }
        boolean unlock = true;
        if (t.startsWith("!")) {
            unlock = false;
            t = t.substring(1).trim();
        }
        t = t.replace('\\', '/');
        String key = null;
        int hash = t.indexOf('#');
        if (hash >= 0) {
            key = t.substring(hash + 1).trim();
            t = t.substring(0, hash);
            if (key.isEmpty()) {
                key = null;
            }
        }
        String path = rulePath(t.trim());
        if (path == null) {
            return null;
        }
        return new Rule(unlock, glob(path), key);
    }

    private static String rulePath(String raw) {
        boolean folder = raw.endsWith("/");
        StringBuilder sb = new StringBuilder();
        for (String part : raw.split("/")) {
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
        if (sb.length() == 0) {
            return null;
        }
        if (folder) {
            sb.append('/');
        }
        return sb.toString();
    }

    private static Path baseFile(Path dir, String rel) throws Exception {
        return dir.resolve(WORK_DIR).resolve("base").resolve(sha1Of(rel.getBytes(StandardCharsets.UTF_8)));
    }

    private static void saveBase(Path dir, String rel, byte[] bytes) {
        try {
            Path f = baseFile(dir, rel);
            if (Files.isRegularFile(f) && Files.size(f) == bytes.length
                    && Arrays.equals(Files.readAllBytes(f), bytes)) {
                return;
            }
            Files.createDirectories(f.getParent());
            Files.write(f, bytes);
        } catch (Exception e) {
            System.err.println("  ! couldn't keep a copy of " + rel + ": " + e.getMessage());
        }
    }

    private static byte[] readBaseBytes(Path dir, String rel) {
        try {
            Path f = baseFile(dir, rel);
            return Files.isRegularFile(f) ? Files.readAllBytes(f) : null;
        } catch (Exception e) {
            return null;
        }
    }

    private static String readBase(Path dir, String rel) {
        try {
            Path f = baseFile(dir, rel);
            return Files.isRegularFile(f) ? new String(Files.readAllBytes(f), StandardCharsets.UTF_8) : null;
        } catch (Exception e) {
            return null;
        }
    }

    private static byte[] mergeSettings(Path dir, String rel, byte[] packBytes, byte[] diskBytes,
            List<String> keys) {
        String pack = new String(packBytes, StandardCharsets.UTF_8);
        String disk = new String(diskBytes, StandardCharsets.UTF_8);
        Map<String, int[]> packSpans = settingSpans(rel, pack);
        Map<String, int[]> diskSpans = settingSpans(rel, disk);
        if (packSpans == null || diskSpans == null) {
            return null;
        }
        String base = readBase(dir, rel);
        Map<String, int[]> baseSpans = base == null ? null : settingSpans(rel, base);
        List<int[]> edits = new ArrayList<int[]>();
        final List<String> values = new ArrayList<String>();
        for (String key : keys) {
            int[] p = packSpans.get(key);
            int[] d = diskSpans.get(key);
            if (p == null || d == null) {
                continue;
            }
            String mine = disk.substring(d[0], d[1]);
            if (baseSpans != null) {
                int[] b = baseSpans.get(key);
                if (b != null && base.substring(b[0], b[1]).equals(mine)) {
                    continue;
                }
            }
            if (pack.substring(p[0], p[1]).equals(mine)) {
                continue;
            }
            edits.add(new int[] { p[0], p[1], values.size() });
            values.add(mine);
        }
        Collections.sort(edits, new java.util.Comparator<int[]>() {
            public int compare(int[] a, int[] b) {
                return b[0] - a[0];
            }
        });
        StringBuilder sb = new StringBuilder(pack);
        for (int[] e : edits) {
            sb.replace(e[0], e[1], values.get(e[2]));
        }
        return sb.toString().getBytes(StandardCharsets.UTF_8);
    }

    private static byte[] forceSettings(String rel, byte[] packBytes, byte[] diskBytes, List<String> keys) {
        String pack = new String(packBytes, StandardCharsets.UTF_8);
        String disk = new String(diskBytes, StandardCharsets.UTF_8);
        Map<String, int[]> packSpans = settingSpans(rel, pack);
        Map<String, int[]> diskSpans = settingSpans(rel, disk);
        if (packSpans == null || diskSpans == null) {
            return null;
        }
        List<int[]> edits = new ArrayList<int[]>();
        final List<String> values = new ArrayList<String>();
        for (String key : keys) {
            int[] p = packSpans.get(key);
            int[] d = diskSpans.get(key);
            if (p == null || d == null) {
                continue;
            }
            String theirs = pack.substring(p[0], p[1]);
            if (theirs.equals(disk.substring(d[0], d[1]))) {
                continue;
            }
            edits.add(new int[] { d[0], d[1], values.size() });
            values.add(theirs);
        }
        if (edits.isEmpty()) {
            return null;
        }
        Collections.sort(edits, new java.util.Comparator<int[]>() {
            public int compare(int[] a, int[] b) {
                return b[0] - a[0];
            }
        });
        StringBuilder sb = new StringBuilder(disk);
        for (int[] e : edits) {
            sb.replace(e[0], e[1], values.get(e[2]));
        }
        return sb.toString().getBytes(StandardCharsets.UTF_8);
    }

    private static Map<String, int[]> settingSpans(String rel, String text) {
        String lower = rel.toLowerCase(java.util.Locale.ROOT);
        String name = lower.substring(lower.lastIndexOf('/') + 1);
        if (name.matches("options(of|shaders)?\\.txt")) {
            return lineSpans(text, false);
        }
        int dot = name.lastIndexOf('.');
        String ext = dot < 0 ? "" : name.substring(dot + 1);
        if (ext.equals("json") || ext.equals("json5")) {
            JsonSpans scan = new JsonSpans(text);
            scan.value("");
            return scan.out;
        }
        if (ext.equals("toml")) {
            return lineSpans(text, true);
        }
        if (ext.equals("properties") || ext.equals("cfg") || ext.equals("ini") || ext.equals("conf")) {
            return lineSpans(text, false);
        }
        if (ext.equals("txt") && looksLikeSettings(text)) {
            return lineSpans(text, false);
        }
        return null;
    }

    private static boolean looksLikeSettings(String text) {
        boolean found = false;
        for (String raw : text.split("\n")) {
            String line = raw.trim();
            if (line.isEmpty() || line.startsWith("#") || line.startsWith("//") || line.startsWith(";")) {
                continue;
            }
            if (!line.matches("[A-Za-z0-9_.\\-]+\\s*[:=].*")) {
                return false;
            }
            found = true;
        }
        return found;
    }

    private static int eqOutside(String ln) {
        char q = 0;
        for (int i = 0; i < ln.length(); i++) {
            char c = ln.charAt(i);
            if (q != 0) {
                if (c == '\\') {
                    i++;
                } else if (c == q) {
                    q = 0;
                }
                continue;
            }
            if (c == '"' || c == '\'') {
                q = c;
            } else if (c == '=') {
                return i;
            } else if (c == '#') {
                return -1;
            }
        }
        return -1;
    }

    private static int propSep(String ln) {
        if (ln.trim().matches("[A-Za-z]:[^=]*=.*")) {
            return ln.indexOf('=');
        }
        for (int i = 0; i < ln.length(); i++) {
            char c = ln.charAt(i);
            if (c == '=' || c == ':') {
                return i;
            }
        }
        return -1;
    }

    private static String stripQuotes(String key) {
        return key.replaceAll("^[\"']|[\"']$", "");
    }

    private static Map<String, int[]> lineSpans(String text, boolean toml) {
        Map<String, int[]> out = new LinkedHashMap<String, int[]>();
        String table = "";
        List<String> categories = new ArrayList<String>();
        int offset = 0;
        String[] lines = text.split("\n", -1);
        for (String ln : lines) {
            int lineStart = offset;
            offset += ln.length() + 1;
            String t = ln.trim();
            if (t.isEmpty()) {
                continue;
            }
            if (t.startsWith("[")) {
                if (toml || t.endsWith("]")) {
                    table = t.replaceAll("^\\[+", "").replaceAll("\\]+.*$", "").trim();
                    continue;
                }
            }
            if (!toml && t.endsWith("{") && t.indexOf('=') < 0) {
                categories.add(stripQuotes(t.substring(0, t.length() - 1).trim()));
                table = join(categories);
                continue;
            }
            if (!toml && t.equals("}")) {
                if (!categories.isEmpty()) {
                    categories.remove(categories.size() - 1);
                }
                table = join(categories);
                continue;
            }
            if (t.startsWith("#") || t.startsWith("!") || t.startsWith(";")) {
                continue;
            }
            int sep = toml ? eqOutside(ln) : propSep(ln);
            if (sep < 0) {
                continue;
            }
            String key = stripQuotes(ln.substring(0, sep).trim());
            if (key.isEmpty()) {
                continue;
            }
            int vs = sep + 1;
            while (vs < ln.length() && (ln.charAt(vs) == ' ' || ln.charAt(vs) == '\t')) {
                vs++;
            }
            int ve;
            char first = vs < ln.length() ? ln.charAt(vs) : 0;
            if (toml && (first == '"' || first == '\'')) {
                ve = vs + 1;
                while (ve < ln.length()) {
                    if (ln.charAt(ve) == '\\') {
                        ve += 2;
                        continue;
                    }
                    if (ln.charAt(ve) == first) {
                        ve++;
                        break;
                    }
                    ve++;
                }
                ve = Math.min(ve, ln.length());
            } else {
                int limit = ln.length();
                if (toml) {
                    char q = 0;
                    for (int k = vs; k < ln.length(); k++) {
                        char c = ln.charAt(k);
                        if (q != 0) {
                            if (c == '\\') {
                                k++;
                            } else if (c == q) {
                                q = 0;
                            }
                            continue;
                        }
                        if (c == '"' || c == '\'') {
                            q = c;
                        } else if (c == '#') {
                            limit = k;
                            break;
                        }
                    }
                }
                ve = limit;
                while (ve > vs && (ln.charAt(ve - 1) == ' ' || ln.charAt(ve - 1) == '\t'
                        || ln.charAt(ve - 1) == '\r')) {
                    ve--;
                }
            }
            String id = table.isEmpty() ? key : table + "." + key;
            if (!out.containsKey(id)) {
                out.put(id, new int[] { lineStart + vs, lineStart + ve });
            }
        }
        return out;
    }

    private static String join(List<String> parts) {
        StringBuilder sb = new StringBuilder();
        for (String p : parts) {
            if (sb.length() > 0) {
                sb.append('.');
            }
            sb.append(p);
        }
        return sb.toString();
    }

    private static final class JsonSpans {
        private final String s;
        private final Map<String, int[]> out = new LinkedHashMap<String, int[]>();
        private int i;

        private JsonSpans(String s) {
            this.s = s;
        }

        private void ws() {
            while (i < s.length()) {
                char c = s.charAt(i);
                if (Character.isWhitespace(c) || c == '\uFEFF') {
                    i++;
                } else if (c == '/' && i + 1 < s.length() && s.charAt(i + 1) == '/') {
                    while (i < s.length() && s.charAt(i) != '\n') {
                        i++;
                    }
                } else if (c == '/' && i + 1 < s.length() && s.charAt(i + 1) == '*') {
                    int end = s.indexOf("*/", i + 2);
                    i = end < 0 ? s.length() : end + 2;
                } else {
                    return;
                }
            }
        }

        private void value(String path) {
            ws();
            if (i >= s.length()) {
                return;
            }
            char c = s.charAt(i);
            if (c == '{') {
                i++;
                object(path);
            } else if (c == '[') {
                i++;
                array(path);
            } else {
                int start = i;
                scalar();
                if (!path.isEmpty() && !out.containsKey(path)) {
                    out.put(path, new int[] { start, i });
                }
            }
        }

        private void object(String path) {
            while (i < s.length()) {
                ws();
                if (i >= s.length()) {
                    return;
                }
                char c = s.charAt(i);
                if (c == '}') {
                    i++;
                    return;
                }
                if (c == ',') {
                    i++;
                    continue;
                }
                int before = i;
                String key = key();
                ws();
                if (i < s.length() && (s.charAt(i) == ':' || s.charAt(i) == '=')) {
                    i++;
                }
                value(path.isEmpty() ? key : path + "." + key);
                if (i == before) {
                    i++;
                }
            }
        }

        private void array(String path) {
            int index = 0;
            while (i < s.length()) {
                ws();
                if (i >= s.length()) {
                    return;
                }
                char c = s.charAt(i);
                if (c == ']') {
                    i++;
                    return;
                }
                if (c == ',') {
                    i++;
                    continue;
                }
                int before = i;
                value(path.isEmpty() ? String.valueOf(index) : path + "." + index);
                index++;
                if (i == before) {
                    i++;
                }
            }
        }

        private String key() {
            char c = s.charAt(i);
            if (c == '"' || c == '\'') {
                return string();
            }
            int start = i;
            while (i < s.length()) {
                char k = s.charAt(i);
                if (k == ':' || k == '=' || k == ',' || k == '}' || Character.isWhitespace(k)) {
                    break;
                }
                i++;
            }
            return s.substring(start, i);
        }

        private String string() {
            char q = s.charAt(i++);
            StringBuilder sb = new StringBuilder();
            while (i < s.length()) {
                char c = s.charAt(i++);
                if (c == q) {
                    return sb.toString();
                }
                if (c != '\\' || i >= s.length()) {
                    sb.append(c);
                    continue;
                }
                char e = s.charAt(i++);
                switch (e) {
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
                        if (i + 4 <= s.length()) {
                            try {
                                sb.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
                                i += 4;
                            } catch (NumberFormatException ignore) {
                                sb.append('u');
                            }
                        }
                        break;
                    default:
                        sb.append(e);
                }
            }
            return sb.toString();
        }

        private void scalar() {
            char c = s.charAt(i);
            if (c == '"' || c == '\'') {
                string();
                return;
            }
            while (i < s.length()) {
                char k = s.charAt(i);
                if (k == ',' || k == '}' || k == ']' || Character.isWhitespace(k)
                        || (k == '/' && i + 1 < s.length() && (s.charAt(i + 1) == '/' || s.charAt(i + 1) == '*'))) {
                    return;
                }
                i++;
            }
        }
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

    private static Map<String, Object> readStateFile(Path dir) {
        Path f = dir.resolve(STATE_FILE);
        if (!Files.exists(f)) {
            return new HashMap<String, Object>();
        }
        try {
            return asMap(Json.parse(new String(Files.readAllBytes(f), StandardCharsets.UTF_8)));
        } catch (Exception e) {
            if (STATE_WARNED.compareAndSet(false, true)) {
                System.err.println("  ! couldn't read " + STATE_FILE + ": "
                        + (e instanceof IOException ? e.getMessage() : "it isn't valid JSON"));
            }
            return new HashMap<String, Object>();
        }
    }

    private static Map<String, String> loadState(Path dir) {
        Map<String, String> out = new LinkedHashMap<String, String>();
        Object files = readStateFile(dir).get("files");
        if (files instanceof Map) {
            for (Map.Entry<String, Object> e : asMap(files).entrySet()) {
                String sha = str(e.getValue());
                if (sha != null && !sha.isEmpty()) {
                    out.put(e.getKey(), sha.toLowerCase());
                }
            }
        } else {
            for (Object p : asList(files)) {
                String rel = safeRel(str(p));
                if (rel == null) {
                    continue;
                }
                String disk = diskSha(dir.resolve(rel));
                if (disk != null) {
                    out.put(rel, disk);
                }
            }
        }
        return out;
    }

    private static Map<String, String> loadStats(Path dir) {
        Map<String, String> out = new LinkedHashMap<String, String>();
        for (Map.Entry<String, Object> e : asMap(readStateFile(dir).get("stat")).entrySet()) {
            String v = str(e.getValue());
            if (v != null && !v.isEmpty()) {
                out.put(e.getKey(), v);
            }
        }
        return out;
    }

    private static Meta loadMeta(Path dir) {
        Map<String, Object> st = readStateFile(dir);
        return new Meta(str(st.get("tag")), str(st.get("etag")));
    }

    private static void saveState(Path dir, Map<String, String> state, String tag, String etag,
            Map<String, String> stats) throws IOException {
        if (tag == null) {
            Meta m = loadMeta(dir);
            tag = m.tag;
            etag = m.etag;
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
            if (code == 401 && token != null && !token.isEmpty()) {
                c.disconnect();
                try {
                    return fetchBytes(url, null, accept);
                } catch (IOException e) {
                    throw new IOException("HTTP 401 for " + current);
                }
            }
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
