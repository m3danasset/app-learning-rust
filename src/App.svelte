<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  const menus = { admin: ["students", "courses", "accounts", "server"], teacher: ["students", "courses"], pb: [], trial: [] };
  const labels = { students: "Murid", courses: "Kursus", accounts: "Akun", server: "Server" };
  const titles = { admin: "Panel Admin", teacher: "Panel Guru", pb: "Panel PB", trial: "Panel Trial" };
  const descs = { students: "Lihat dan kelola data murid.", courses: "Lihat dan kelola daftar kursus.", accounts: "Akun yang bisa masuk ke panel.", server: "Alamat server untuk aplikasi murid." };
  const colors = ["#f97316", "#3b82f6", "#a855f7", "#22c55e"];
  const ic = {
    dashboard: "M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10",
    students: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75",
    courses: "M2 4h6a4 4 0 0 1 4 4v13a3 3 0 0 0-3-3H2zM22 4h-6a4 4 0 0 0-4 4v13a3 3 0 0 1 3-3h7z",
    accounts: "M3 5h18v14H3zM8 11h.01M5 16c.6-1.5 1.8-2 3-2s2.4.5 3 2M14 9h4M14 13h4",
    server: "M3 4h18v6H3zM3 14h18v6H3zM7 7h.01M7 17h.01",
    panel: "M3 4h18v16H3zM9 4v16",
    chev: "M7 15l5 5 5-5M7 9l5-5 5 5",
  };

  // kept in sessionStorage so Ctrl+R reload stays logged in; cleared when the app closes
  let user = $state(JSON.parse(sessionStorage.getItem("user") ?? "null"));
  $effect(() => { user ? sessionStorage.setItem("user", JSON.stringify(user)) : sessionStorage.removeItem("user"); });
  let tab = $state("dashboard");
  let err = $state("");
  let url = $state("");
  let collapsed = $state(true);
  let menuOpen = $state(false);
  let track = $state();
  let atStart = $state(true);
  let atEnd = $state(false);
  let drag = null;
  let staff = $state([]);
  let students = $state([]);
  let courses = $state([]);
  // trial sees two fixed preview cards instead of the course list
  const trialCards = [
    { id: "modeling", title: "MODELING", description: "Build 3D objects from basic shapes." },
    { id: "animation", title: "ANIMATION", description: "Bring objects and characters to life with motion." },
  ];
  const trial = $derived(user?.role === "trial");
  const cards = $derived(trial ? trialCards : courses);
  let f = $state({ name: "", username: "", title: "", description: "" });
  const tr = {
    id: {
      welcome: "Selamat datang!", hello: "Halo", pick: "Pilih kursus untuk mulai belajar.", none: "Belum ada menu untuk panel ini.", logout: "Logout", noCourses: "Belum ada kursus.",
      labels: { students: "Murid", courses: "Kursus", accounts: "Akun", server: "Server" },
      titles: { admin: "Panel Admin", teacher: "Panel Guru", pb: "Panel PB", trial: "Panel Trial" },
    },
    en: {
      welcome: "Welcome!", hello: "Hi", pick: "Pick a course to start learning.", none: "No menus are available for this panel yet.", logout: "Log out", noCourses: "No courses yet.",
      labels: { students: "Students", courses: "Courses", accounts: "Accounts", server: "Server" },
      titles: { admin: "Admin Panel", teacher: "Teacher Panel", pb: "PB Panel", trial: "Trial Panel" },
    },
  };
  const lang = $derived(user && (user.role === "pb" || user.role === "trial") ? "en" : "id");
  const t = $derived(tr[lang]);
  const canEdit = $derived(user?.role === "admin" || user?.role === "teacher");
  const initials = $derived((user?.name ?? "").split(" ").map((w) => w[0]).join("").slice(0, 2).toUpperCase());

  const run = async (cmd, args) => {
    err = "";
    try { return await invoke(cmd, args); } catch (e) { err = String(e); }
  };
  const load = async () => {
    students = (await run("list_students")) ?? [];
    courses = (await run("list_courses")) ?? [];
    if (user?.role === "admin") staff = (await run("list_staff")) ?? [];
  };
  const go = (t) => { tab = t; err = ""; menuOpen = false; };
  const info = (m) => m === "students" ? `${students.length} murid` : m === "courses" ? `${courses.length} kursus` : m === "accounts" ? `${staff.length} akun` : "Aktif";
  const logout = () => { user = null; err = ""; menuOpen = false; tab = "dashboard"; };

  const edges = () => {
    if (!track) return;
    atStart = track.scrollLeft < 4;
    atEnd = track.scrollLeft + track.clientWidth >= track.scrollWidth - 4;
  };
  // public/courses/<slug>.<ext>; missing files simply don't paint, so the color gradient shows through
  const img = (title) => {
    const s = title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
    return ["webp", "jpg", "png"].map((e) => `url("/courses/${s}.${e}") center/cover no-repeat`).join(",");
  };
  // one page = the visible cards; snap keeps the landing card aligned
  const slide = (d) => track?.scrollBy({ left: d * (track.children[1]?.offsetLeft - track.children[0]?.offsetLeft || 0) * +getComputedStyle(track).getPropertyValue("--n"), behavior: "smooth" });
  const down = (e) => { if (e.pointerType === "mouse") drag = { x: e.clientX, left: track.scrollLeft, moved: false }; };
  const move = (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    if (Math.abs(dx) > 4) { drag.moved = true; track.classList.add("dragging"); }
    if (drag.moved) track.scrollLeft = drag.left - dx;
  };
  const up = () => { drag = null; track?.classList.remove("dragging"); };
  $effect(() => { courses.length; track; edges(); });

  onMount(async () => { if (user) load(); url = await invoke("server_info"); });

  async function auth() {
    const u = await run("login", { username: f.username });
    if (u) { user = u; tab = "dashboard"; f.username = ""; load(); }
  }
  async function addStudent() {
    await run("add_student", { name: f.name, username: f.username });
    if (!err) { f.name = f.username = ""; load(); }
  }
  async function addCourse() {
    await run("add_course", { title: f.title, description: f.description });
    if (!err) { f.title = f.description = ""; load(); }
  }
  const del = async (cmd, id) => { await run(cmd, { id }); load(); };
</script>

<svelte:window onresize={edges} onkeydown={(e) => { if ((e.ctrlKey && e.key.toLowerCase() === "r") || e.key === "F5") { e.preventDefault(); location.reload(); } }} />

<button class="x" onclick={() => run("quit")} aria-label="Exit">
  <svg viewBox="0 0 24 24"><path d="M5 5l14 14M19 5L5 19" /></svg>
</button>

{#if !user}
  <div class="login-bg">
    <img class="logo" src="/logo.png" alt="M3DAN Studios" />
    <form onsubmit={(e) => { e.preventDefault(); auth(); }}>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="id" placeholder="Enter Your ID" autocomplete="off" autofocus bind:value={f.username} />
      <p class="login-err">{err}</p>
    </form>
  </div>
{:else}
  <div class="layout" class:collapsed>
    <aside class="side">
      <div class="top">
        <img class="mini" src="/logo.png" alt="M3DAN Studios" />
        <button class="ico" onclick={() => (collapsed = !collapsed)} aria-label="Toggle sidebar"><svg viewBox="0 0 24 24"><path d={ic.panel} /></svg></button>
      </div>
      <button class="nav" class:on={tab === "dashboard"} onclick={() => go("dashboard")}><svg viewBox="0 0 24 24"><path d={ic.dashboard} /></svg><span>Dashboard</span></button>
      {#each menus[user.role] ?? [] as m}
        <button class="nav" class:on={tab === m} onclick={() => go(m)}><svg viewBox="0 0 24 24"><path d={ic[m]} /></svg><span>{t.labels[m]}</span></button>
      {/each}
      <div class="grow"></div>
      <div class="acct">
        {#if menuOpen}<div class="pop"><button onclick={logout}>{t.logout}</button></div>{/if}
        <button class="me" onclick={() => (menuOpen = !menuOpen)}>
          <span class="av">{initials}</span>
          <span class="who"><b>{user.name}</b><small>{t.titles[user.role]}</small></span>
          <svg viewBox="0 0 24 24"><path d={ic.chev} /></svg>
        </button>
      </div>
    </aside>

    <main class:home={tab === "dashboard"}>
      {#if err}<p class="err">{err}</p>{/if}
      {#if user.role === "trial" && tab !== "dashboard"}<p class="mute">Mode trial: data hanya bisa dilihat.</p>{/if}

      {#if tab === "dashboard"}
        <h1 class="hero">{t.welcome}</h1>
        <p class="mute sub">{t.hello}, {user.name}. {t.pick}</p>
        {#if cards.length}
          <div class="carousel" class:fixed={trial}>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="track" bind:this={track} onscroll={edges} onpointerdown={down} onpointermove={move} onpointerup={up} onpointerleave={up}>
              {#each cards as c, i (c.id)}
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <article class="ccard" tabindex="0" style="--c:{colors[i % 4]};--img:{img(c.title)}">
                  <strong>{c.title}</strong>
                  <p><span>{c.description}</span></p>
                </article>
              {/each}
            </div>
            {#if !trial}
            <div class="nav2">
              <button class="round" disabled={atStart} onclick={() => slide(-1)} aria-label="Previous"><svg viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg></button>
              <button class="round" disabled={atEnd} onclick={() => slide(1)} aria-label="Next"><svg viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg></button>
            </div>
            {/if}
          </div>
        {:else}
          <p class="mute">{t.noCourses}</p>
        {/if}
      {:else if tab === "students"}
        <h1>Data murid</h1>
        {#if canEdit}
          <div class="card row">
            <input placeholder="Nama murid" bind:value={f.name} />
            <input placeholder="ID murid" bind:value={f.username} />
            <button class="btn" onclick={addStudent}>Tambah murid</button>
          </div>
        {/if}
        <div class="card">
          <table>
            <thead><tr><th>Nama</th><th>ID</th><th></th></tr></thead>
            <tbody>
              {#each students as s (s.id)}
                <tr>
                  <td>{s.name}</td><td>{s.username}</td>
                  <td>{#if canEdit}<button class="btn del" onclick={() => del("delete_student", s.id)}>Hapus</button>{/if}</td>
                </tr>
              {:else}
                <tr><td colspan="3" class="mute">Belum ada murid.</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else if tab === "courses"}
        <h1>Data kursus</h1>
        {#if canEdit}
          <div class="card row">
            <input placeholder="Judul kursus" bind:value={f.title} />
            <input placeholder="Deskripsi singkat" bind:value={f.description} />
            <button class="btn" onclick={addCourse}>Tambah kursus</button>
          </div>
        {/if}
        <div class="card">
          <table>
            <thead><tr><th>Judul</th><th>Deskripsi</th><th></th></tr></thead>
            <tbody>
              {#each courses as c (c.id)}
                <tr>
                  <td>{c.title}</td><td class="mute">{c.description}</td>
                  <td>{#if canEdit}<button class="btn del" onclick={() => del("delete_course", c.id)}>Hapus</button>{/if}</td>
                </tr>
              {:else}
                <tr><td colspan="3" class="mute">Belum ada kursus.</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else if tab === "accounts"}
        <h1>Akun</h1>
        <div class="card">
          <table>
            <thead><tr><th>Nama</th><th>ID</th><th>Peran</th></tr></thead>
            <tbody>
              {#each staff as s (s.id)}
                <tr><td>{s.name}</td><td>{s.username}</td><td class="mute">{s.role}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <h1>Server</h1>
        <div class="card">
          <p class="mute">Alamat server untuk aplikasi murid</p>
          <p class="url">{url}</p>
          <p class="mute">Server aktif selama aplikasi ini terbuka. Klik Izinkan jika Windows Firewall bertanya. Endpoint: /api/ping dan /api/courses.</p>
        </div>
      {/if}
    </main>
  </div>
{/if}
