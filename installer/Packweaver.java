import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.atomic.AtomicInteger;

public class Packweaver {
    private static final HttpClient HTTP = HttpClient.newBuilder()
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build();

    private static final String UA = "packweave-installer/0.1.0 (+https://github.com/packweave)";

    public static void main(String[] args) {
        try {
            run(args);
        } catch (Exception e) {
            System.err.println("packweaver: " + e.getMessage());
            System.exit(1);
        }
    }

    private static void run(String[] args) throws Exception {
        String url = null;
        Path dir = null;
        boolean server = false;
        for (String a : args) {
            if (a.equals("--server")) {
                server = true;
            } else if (a.startsWith("http://") || a.startsWith("https://")) {
                url = a;
            } else {
                dir = Paths.get(a);
            }
        }
        if (dir == null) {
            dir = Paths.get(".").toAbsolutePath().normalize();
        }
        Files.createDirectories(dir);
        if (url == null) {
            Path urlFile = dir.resolve("modpack.url");
            if (Files.exists(urlFile)) {
                url = Files.readString(urlFile).trim();
            }
        }
        if (url == null || url.isBlank()) {
            url = prompt("Pack URL: ");
        }
        if (url == null || url.isBlank()) {
            throw new IOException("No pack URL provided.");
        }
        url = url.replaceAll("/+$", "");
        Files.writeString(dir.resolve("modpack.url"), url + "\n");

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
        List<String> contentPaths = new ArrayList<>();
        List<String[]> tasks = new ArrayList<>();
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
        final boolean srv = server;
        ExecutorService metaPool = Executors.newFixedThreadPool(8);
        List<Future<String[]>> metaFutures = new ArrayList<>();
        for (String cp : contentPaths) {
            metaFutures.add(metaPool.submit(() -> {
                try {
                    Map<String, Object> m = asMap(Json.parse(get(base + "/" + enc(cp))));
                    Map<String, Object> src = asMap(asMap(m.get("sources")).get(str(m.get("preferred"))));
                    String dl = str(src.get("downloadUrl"));
                    String fn = str(src.get("filename"));
                    if (dl == null || dl.isEmpty() || fn == null || fn.isEmpty()) {
                        return null;
                    }
                    String side = srv ? str(m.get("serverSide")) : str(m.get("clientSide"));
                    if ("unsupported".equals(side)) {
                        return null;
                    }
                    String[] segs = cp.split("/");
                    String sub = segs.length >= 2 ? segs[1] : "mods";
                    String folder = "shaders".equals(sub) ? "shaderpacks" : sub;
                    return new String[] { folder + "/" + fn, dl, str(src.get("sha1")) };
                } catch (Exception e) {
                    System.err.println("  ! " + cp + ": " + e.getMessage());
                    return null;
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

        LinkedHashMap<String, String> desired = new LinkedHashMap<>();
        List<String[]> downloads = new ArrayList<>();
        for (String[] t : tasks) {
            if (desired.containsKey(t[0])) {
                continue;
            }
            desired.put(t[0], t[2]);
            downloads.add(t);
        }

        Path stateFile = dir.resolve(".packweaver-state.json");
        Set<String> prev = new LinkedHashSet<>();
        if (Files.exists(stateFile)) {
            try {
                Map<String, Object> st = asMap(Json.parse(Files.readString(stateFile)));
                for (Object p : asList(st.get("files"))) {
                    prev.add(str(p));
                }
            } catch (Exception ignore) {
            }
        }
        for (String old : prev) {
            if (!desired.containsKey(old)) {
                try {
                    Files.deleteIfExists(dir.resolve(old));
                } catch (Exception ignore) {
                }
            }
        }

        final Path root = dir;
        ExecutorService pool = Executors.newFixedThreadPool(8);
        List<Future<?>> futures = new ArrayList<>();
        int total = downloads.size();
        AtomicInteger done = new AtomicInteger();
        AtomicInteger failed = new AtomicInteger();
        for (String[] d : downloads) {
            final String rel = d[0];
            final String src = d[1];
            final String sha1 = d[2];
            futures.add(pool.submit(() -> {
                try {
                    Path out = root.resolve(rel);
                    if (sha1 != null && !sha1.isEmpty() && Files.exists(out)
                            && sha1.equalsIgnoreCase(sha1Of(Files.readAllBytes(out)))) {
                        done.incrementAndGet();
                        return null;
                    }
                    Files.createDirectories(out.getParent());
                    byte[] bytes = getBytes(src);
                    if (sha1 != null && !sha1.isEmpty()) {
                        String got = sha1Of(bytes);
                        if (!sha1.equalsIgnoreCase(got)) {
                            throw new IOException("checksum mismatch");
                        }
                    }
                    Files.write(out, bytes);
                    int n = done.incrementAndGet();
                    System.out.println("  [" + n + "/" + total + "] " + rel);
                } catch (Exception e) {
                    failed.incrementAndGet();
                    System.err.println("  ! " + rel + ": " + e.getMessage());
                }
                return null;
            }));
        }
        for (Future<?> f : futures) {
            f.get();
        }
        pool.shutdown();

        StringBuilder sb = new StringBuilder("{\"files\":[");
        boolean first = true;
        for (String k : desired.keySet()) {
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append(Json.quote(k));
        }
        sb.append("]}\n");
        Files.writeString(stateFile, sb.toString());

        if (failed.get() > 0) {
            System.out.println("Done with " + failed.get() + " failures, " + desired.size()
                    + " files tracked in " + dir);
        } else {
            System.out.println("Up to date, " + desired.size() + " files in " + dir);
        }
    }

    private static String enc(String path) {
        StringBuilder sb = new StringBuilder();
        for (String seg : path.split("/")) {
            if (sb.length() > 0) {
                sb.append('/');
            }
            sb.append(URLEncoder.encode(seg, StandardCharsets.UTF_8).replace("+", "%20"));
        }
        return sb.toString();
    }

    private static String get(String url) throws Exception {
        HttpResponse<String> r = HTTP.send(
                HttpRequest.newBuilder(URI.create(url)).header("User-Agent", UA).GET().build(),
                HttpResponse.BodyHandlers.ofString());
        if (r.statusCode() / 100 != 2) {
            throw new IOException("HTTP " + r.statusCode() + " for " + url);
        }
        return r.body();
    }

    private static byte[] getBytes(String url) throws Exception {
        HttpResponse<byte[]> r = HTTP.send(
                HttpRequest.newBuilder(URI.create(url)).header("User-Agent", UA).GET().build(),
                HttpResponse.BodyHandlers.ofByteArray());
        if (r.statusCode() / 100 != 2) {
            throw new IOException("HTTP " + r.statusCode());
        }
        return r.body();
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
        return o instanceof Map ? (Map<String, Object>) o : new HashMap<>();
    }

    @SuppressWarnings("unchecked")
    private static List<Object> asList(Object o) {
        return o instanceof List ? (List<Object>) o : new ArrayList<>();
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
            Map<String, Object> m = new LinkedHashMap<>();
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
            List<Object> a = new ArrayList<>();
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
