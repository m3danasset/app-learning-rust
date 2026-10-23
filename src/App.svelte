<script>
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  const menus = { admin: ["students", "courses", "accounts", "server"], teacher: ["students", "courses"], pb: [], trial: [] };
  const labels = { students: "Murid", courses: "Kursus", accounts: "Akun", server: "Server" };
  const titles = { admin: "Panel Admin", teacher: "Panel Guru", pb: "Panel PB", trial: "Panel Trial" };
  const descs = { students: "Lihat dan kelola data murid.", courses: "Lihat dan kelola daftar kursus.", accounts: "Akun yang bisa masuk ke panel.", server: "Alamat server untuk aplikasi murid." };
  const qa = { students: "Tambah Murid", courses: "Tambah Kursus", accounts: "Kelola Akun" };
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
  const logout = () => { user = null; err = ""; menuOpen = false; tab = "dashboard"; playing = null; };

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
  // snap + smooth scrolling swallow small scrollLeft nudges, so one wheel notch = one page, throttled while it animates
  let wheelAt = 0;
  const wheel = (e) => {
    const d = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (!d || Date.now() - wheelAt < 450) return;
    wheelAt = Date.now();
    slide(Math.sign(d));
  };
  // lessons live outside the bundle in videos/<course-slug>/: "<n>. <title>.mp4" + "<title>_IND.vtt" / "_ENG.vtt"
  const slug = (title) => title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
  let playing = $state(null);
  let lessons = $state([]);
  let cur = $state(0);
  let subs = $state([]);
  const lesson = $derived(lessons[cur]);
  // dev panel (Ctrl+Shift+D): floating window that switches the app window between fullscreen and fixed sizes
  const views = [["fullscreen", "Fullscreen"], ["768", "768p"], ["1080", "1080p"], ["1440", "1440p"]];
  let devOpen = $state(false);
  let view = $state("fullscreen");
  let vw = $state(0);
  let vh = $state(0);
  let dev = $state({ x: 24, y: 24 });
  let devDrag = null;
  const setView = async (m) => { await run("set_view", { mode: m }); if (!err) view = m; };
  const devDown = (e) => { devDrag = { dx: e.clientX - dev.x, dy: e.clientY - dev.y }; e.currentTarget.setPointerCapture(e.pointerId); };
  const devMove = (e) => { if (devDrag) dev = { x: e.clientX - devDrag.dx, y: e.clientY - devDrag.dy }; };
  // per-student notes, kept in localStorage per lesson
  const noteKey = $derived(user && lesson ? `note:${user.id}:${lesson.video}` : null);
  let note = $state("");
  $effect(() => { try { note = (noteKey && localStorage.getItem(noteKey)) || ""; } catch { note = ""; } });
  const saveNote = () => { try { noteKey && localStorage.setItem(noteKey, note); } catch {} };
  let copied = $state(false);
  const copyTitle = async () => {
    try { await navigator.clipboard.writeText(lesson.title); } catch { return; }
    copied = true;
    setTimeout(() => (copied = false), 1500);
  };
  let dragged = false;
  const open =async (c) => {
    if (dragged) { dragged = false; return; }
    playing = c; lessons = []; cur = 0;
    lessons = (await run("list_lessons", { course: slug(c.title) })) ?? [];
  };
  // subtitles go through a Blob so the video never needs cross-origin track access
  $effect(() => {
    const l = lesson, urls = [];
    subs = [];
    if (!l) return;
    (async () => {
      const out = [];
      for (const [p, srclang, label] of [[l.sub_id, "id", "Indonesia"], [l.sub_en, "en", "English"]]) {
        if (!p) continue;
        const txt = await invoke("read_sub", { path: p }).catch(() => null);
        if (txt == null) continue;
        const src = URL.createObjectURL(new Blob([txt], { type: "text/vtt" }));
        urls.push(src); out.push({ src, srclang, label });
      }
      if (lesson === l) subs = out;
    })();
    return () => urls.forEach(URL.revokeObjectURL);
  });
  const down = (e) => { dragged = false; if (e.pointerType === "mouse") drag = { x: e.clientX, left: track.scrollLeft, moved: false }; };
  const move = (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    if (Math.abs(dx) > 4) { drag.moved = true; track.classList.add("dragging"); }
    if (drag.moved) track.scrollLeft = drag.left - dx;
  };
  const up = () => { dragged = !!drag?.moved; drag = null; track?.classList.remove("dragging"); };
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

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} onresize={edges} onkeydown={(e) => { if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "d") { e.preventDefault(); devOpen = !devOpen; return; } if ((e.ctrlKey && e.key.toLowerCase() === "r") || e.key === "F5") { e.preventDefault(); location.reload(); } }} />

{#if devOpen}
  <div class="dev" style="left:{dev.x}px;top:{dev.y}px">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dev-head" onpointerdown={devDown} onpointermove={devMove} onpointerup={() => (devDrag = null)}>
      <b>Dev Mode</b>
      <button class="ico" onclick={() => (devOpen = false)} aria-label="Close"><svg viewBox="0 0 24 24"><path d="M5 5l14 14M19 5L5 19" /></svg></button>
    </div>
    <div class="dev-body">
      <small class="mute">Resolusi window</small>
      {#each views as [m, label]}
        <button class="dev-opt" class:on={view === m} onclick={() => setView(m)}>{label}</button>
      {/each}
      <small class="mute">Viewport: {vw} × {vh}</small>
    </div>
  </div>
{/if}

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

    <main class:home={tab === "dashboard" && !canEdit && !playing}>
      {#if err}<p class="err">{err}</p>{/if}
      {#if user.role === "trial" && tab !== "dashboard"}<p class="mute">Mode trial: data hanya bisa dilihat.</p>{/if}

      {#if tab === "dashboard" && canEdit}
        <header class="dash-head">
          <div>
            <h1 class="hero">Selamat datang, {user.name}</h1>
            <p class="mute">Ringkasan data panel hari ini.</p>
          </div>
          <div class="quick">
            {#each menus[user.role].filter((m) => m !== "server") as m}
              <button class="card qa" onclick={() => go(m)}>
                <svg viewBox="0 0 24 24"><path d={ic[m]} /></svg>
                <span>{qa[m]}</span>
              </button>
            {/each}
          </div>
        </header>

        <div class="stats">
          {#each menus[user.role] as m, i}
            <button class="card stat" onclick={() => go(m)}>
              <span class="badge" style="--c:{colors[i % 4]}"><svg viewBox="0 0 24 24"><path d={ic[m]} /></svg></span>
              <span class="lbl"><small class="mute">{t.labels[m]}</small><b>{m === "students" ? students.length : m === "courses" ? courses.length : m === "accounts" ? staff.length : "Aktif"}</b></span>
            </button>
          {/each}
        </div>

        <div class="panels">
          <section class="card panel">
            <div class="ph"><h2>Murid terbaru</h2><button class="link" onclick={() => go("students")}>Lihat semua</button></div>
            {#each [...students].reverse().slice(0, 5) as s, i (s.id)}
              <div class="li">
                <span class="av" style="background:{colors[i % 4]}">{s.name.split(" ").map((w) => w[0]).join("").slice(0, 2).toUpperCase()}</span>
                <span class="grow2"><b>{s.name}</b><small class="mute">{s.username}</small></span>
              </div>
            {:else}<p class="mute">Belum ada murid.</p>{/each}
          </section>
          <section class="card panel">
            <div class="ph"><h2>Daftar kursus</h2><button class="link" onclick={() => go("courses")}>Lihat semua</button></div>
            {#each courses.slice(0, 5) as c, i (c.id)}
              <div class="li course" style="--c:{colors[i % 4]}">
                <span class="grow2"><b>{c.title}</b><small class="mute">{c.description}</small></span>
              </div>
            {:else}<p class="mute">{t.noCourses}</p>{/each}
          </section>
        </div>
      {:else if tab === "dashboard" && playing}
        <div class="player">
          <div class="pbar">
            <button class="round" onclick={() => (playing = null)} aria-label="Back"><svg viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg></button>
            <h1>{playing.title}</h1>
          </div>
          {#if lesson}
            <div class="pbody">
              <div class="stage">
                <div class="vtitle">
                  <h2>{lesson.title}</h2>
                  <button class="ico" onclick={copyTitle} aria-label="Copy title">
                    <svg viewBox="0 0 24 24"><path d={copied ? "M5 12l5 5 9-10" : "M9 9h11v11H9zM5 15V4h11"} /></svg>
                  </button>
                </div>
                {#key lesson.video}
                  <!-- svelte-ignore a11y_media_has_caption -->
                  <video controls autoplay src={convertFileSrc(lesson.video)} onended={() => cur < lessons.length - 1 && cur++}>
                    {#each subs as s (s.src)}
                      <track kind="subtitles" src={s.src} srclang={s.srclang} label={s.label} default={s.srclang === lang} />
                    {/each}
                  </video>
                {/key}
                <div class="vnav">
                  <button class="nb" disabled={cur === 0} onclick={() => cur--}><svg viewBox="0 0 24 24"><path d="M15 6l-6 6 6 6" /></svg>{lang === "en" ? "Previous" : "Sebelumnya"}</button>
                  <div class="note">
                    <textarea rows="1" spellcheck="false" placeholder={lang === "en" ? "Write a note…" : "Tulis catatan…"} bind:value={note} oninput={saveNote}></textarea>
                  </div>
                  <button class="nb" disabled={cur >= lessons.length - 1} onclick={() => cur++}>{lang === "en" ? "Next" : "Berikutnya"}<svg viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg></button>
                </div>
              </div>
              <ol class="lessons">
                {#each lessons as l, i (l.video)}
                  <li><button class:on={i === cur} title={l.title} onclick={() => (cur = i)}><span>{l.title}</span></button></li>
                {/each}
              </ol>
            </div>
          {:else}
            <p class="mute">{lang === "en" ? "No lessons yet." : "Belum ada video."}</p>
          {/if}
        </div>
      {:else if tab === "dashboard"}
        <h1 class="hero">{t.welcome}</h1>
        <p class="mute sub">{t.hello}, {user.name}. {t.pick}</p>
        {#if cards.length}
          <div class="carousel" class:fixed={trial}>
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="track" bind:this={track} onscroll={edges} onwheel={wheel} onpointerdown={down} onpointermove={move} onpointerup={up} onpointerleave={up}>
              {#each cards as c, i (c.id)}
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <article class="ccard" tabindex="0" onclick={() => open(c)} onkeydown={(e) => e.key === "Enter" && open(c)} style="--c:{colors[i % 4]};--img:{img(c.title)}">
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
                  <td>{c.title}</td>
                  <td class="mute">
                    {#if user.role === "admin"}
                      <textarea class="edit" rows="2" value={c.description} onkeydown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); e.currentTarget.blur(); } }} onchange={(e) => run("update_course", { id: c.id, description: e.currentTarget.value }).then(load)}></textarea>
                    {:else}{c.description}{/if}
                  </td>
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
