/**
 * Endmill: a milling cutter lying on its side, its tip toward the viewer. The
 * pointer rolls it about its axis on a spring, so the flutes pass by: each is
 * a cutting edge and, behind it, the dim heel of its gullet. One edge is
 * bright and says where the eye starts. The handle also takes
 * shape(tool): family, helix, flutes and five lengths in mm, each on the 700ms
 * tween, so the drawing follows a tool being described. The proportions are
 * the tool's own and the whole is scaled to one box; a shank the description
 * leaves blank is drawn dotted. The slider is the roll, in degrees.
 *
 * The pattern: scrub. A spring for where the pointer is, tweens for what the
 * tool is, and a body of revolution drawn as the hull of its rings.
 */
const {
  Cam, clamp, fit, hull, open, poly, proj, rad,
  spring, stepS, tdone, tset, tval, tween, disposer, mk, pointer, register,
} = HL;

const BOX_W = 320, BOX_H = 236, REST = 40, RING = 40, STEPS = 24, HELIX = Math.tan(rad(30)), TAU = Math.PI * 2;
// Toward the camera of Cam(45, 0.5), across the axis: a point of the surface shows when its normal leans this way.
const CY = Math.sqrt(0.375), CZ = 0.5;
const NUM = ["d", "lc", "sh", "lt", "tip", "up"];
const DEF = { family: "flat_endmill", helix: "upcut", flutes: 2, guess: false, d: 6, lc: 22, sh: 6, lt: 50, tip: 90, up: 0 };

const pos = (v) => (typeof v === "number" && v > 0 ? v : null);

/** A description, as a form gives it, made whole: what is blank takes a likely value, and `guess` says the shank is one. */
function whole(tool) {
  const d = pos(tool.diameter) ?? DEF.d, lc = pos(tool.fluteLength) ?? d * 3.5;
  const sh = pos(tool.shank) ?? clamp(d, 3, 12), lt = Math.max(pos(tool.overall) ?? lc + Math.max(lc * 1.2, sh * 4), lc * 1.08);
  return {
    family: tool.family ?? DEF.family, helix: tool.helix ?? DEF.helix, flutes: clamp(Math.round(pos(tool.flutes) ?? 2), 1, 8),
    guess: !pos(tool.shank) || !pos(tool.overall), d, lc, sh, lt, tip: clamp(pos(tool.tipAngle) ?? 90, 20, 160), up: pos(tool.upcut) ?? 0,
  };
}

/** The cutter's profile from its tip: [distance along the axis, radius] in mm, convex. */
function profile(family, R, lc, tip) {
  if (family === "ball_nose") {
    const out = [];
    for (let a = 0; a <= 90; a += 15) out.push([R * (1 - Math.cos(rad(a))), Math.max(R * 0.05, R * Math.sin(rad(a)))]);
    return out.concat([[Math.max(lc, R * 1.05), R]]);
  }
  if (family === "v_bit") {
    const h = Math.min(R / Math.tan(rad(tip / 2)), lc * 0.96);
    return [[0, R * 0.05], [h, R], [lc, R]];
  }
  return [[0, R], [lc, R]];
}

/** The radius at distance s from the tip. */
function radius(pf, s) {
  for (let i = 1; i < pf.length; i++) if (s <= pf[i][0]) {
    const [s0, r0] = pf[i - 1], [s1, r1] = pf[i];
    return s1 > s0 ? r0 + ((r1 - r0) * (s - s0)) / (s1 - s0) : r1;
  }
  return pf[pf.length - 1][1];
}

/** How far round a flute has turned at distance s from the tip, in radians, by kind of helix. */
function twist(kind, s, R, lc, up) {
  const k = HELIX / R;
  if (kind === "upcut") return k * s;
  if (kind === "downcut") return -k * s;
  if (kind !== "compression") return 0;
  const su = up > 0 ? Math.min(up, lc) : lc / 2;
  return k * (s < su ? s : 2 * su - s);
}

function mount({ stage, svg, read }, value) {
  const bag = disposer();
  let turn = value, st = { ...DEF }, drawn = "";

  // One camera for every tool: the axis runs through the centre of the frame, and draw() scales the tool to the box.
  const C = Cam(45, 0.5, 1);
  fit(C, [[-1, -1, -1], [1, 1, 1], [1, -1, 0], [-1, 1, 0]], 200, 166);
  const P = proj(C);
  // draw() moves the whole by [ox, oy], so a tool with one end wider than the other still sits in the middle.
  let ox = 0, oy = 0;
  const at = (x, r, t) => { const p = P(x, r * Math.cos(t), r * Math.sin(t)); return [p[0] + ox, p[1] + oy]; };
  const ring = (x, r) => Array.from({ length: RING }, (_, i) => at(x, r, (i / RING) * TAU));

  // Far to near: the shank, the face it shows when it is the wider, the cutter, its end face, the flutes.
  const g = mk("g", {}, svg);
  const shank = mk("path", { class: "sil" }, g), step = mk("path", { class: "nf lo" }, g);
  const body = mk("path", { class: "sil" }, g), face = mk("path", { class: "nf lo" }, g);
  const heels = mk("path", { class: "nf lo" }, g), others = mk("path", { class: "nf" }, g), edge = mk("path", { class: "nf hi" }, g);

  const roll = spring(0), T = {};
  for (const key of NUM) T[key] = tween(DEF[key]);

  /** One line of a flute, from the tip to the end of the cut: the runs of it on the side that shows, and across a flat end. */
  function line(pf, kind, phase, dir, v, k, x1) {
    let d = "", pts = [];
    for (let i = 0; i <= STEPS; i++) {
      const s = (v.lc * i) / STEPS, t = phase + dir * twist(kind, s, v.d / 2, v.lc, v.up);
      if (CY * Math.cos(t) + CZ * Math.sin(t) > 0.03) pts.push(at(x1 - s * k, radius(pf, s) * k, t));
      else { if (pts.length > 1) d += open(pts); pts = []; }
    }
    if (pts.length > 1) d += open(pts);
    return pf[0][1] < v.d / 2 ? d : d + open([at(x1, v.d * 0.09 * k, phase), at(x1, (v.d / 2) * k, phase)]);
  }

  function draw(v, angle) {
    const R = v.d / 2, Rs = v.sh / 2;
    const k = Math.min(BOX_W / (Math.SQRT1_2 * (v.lt + R + Rs)), BOX_H / (0.354 * v.lt + 0.935 * (R + Rs)));
    const x1 = (v.lt * k) / 2, xc = x1 - v.lc * k, x0 = -x1;
    ox = -0.354 * (R - Rs) * k; oy = -0.468 * (R - Rs) * k;
    const pf = profile(st.family, R, v.lc, v.tip);

    shank.setAttribute("d", poly(hull(ring(x0, Rs * k).concat(ring(xc, Rs * k)))));
    shank.setAttribute("class", st.guess ? "sil dash" : "sil");
    step.setAttribute("d", Rs > R ? poly(ring(xc, Math.max(0.5, Rs * k - 1.6))) : "");
    body.setAttribute("d", poly(hull(pf.flatMap(([s, r]) => ring(x1 - s * k, r * k)))));
    face.setAttribute("d", pf[0][1] < R ? "" : poly(ring(x1, Math.max(0.5, R * k - 1.6))));

    const cross = st.family === "diamond_cut", straight = st.family === "v_bit" || st.family === "surfacing";
    const kind = st.family === "compression" ? "compression" : cross ? "upcut" : straight ? "straight" : st.helix;
    // Each flute is its cutting edge and the heel of its gullet, a part of the pitch behind it; a crossed cut has edges only.
    const gullet = (TAU / st.flutes) * 0.42;
    let hi = "", rest = "", heel = "";
    for (const dir of cross ? [1, -1] : [1]) for (let j = 0; j < st.flutes; j++) {
      const phase = angle + (j / st.flutes) * TAU, d = line(pf, kind, phase, dir, v, k, x1);
      if (j === 0 && dir === 1) hi = d; else rest += d;
      if (!cross) heel += line(pf, kind, phase - gullet, dir, v, k, x1);
    }
    edge.setAttribute("d", hi);
    others.setAttribute("d", rest);
    heels.setAttribute("d", heel);
    others.setAttribute("class", kind === "unknown" ? "nf dash" : "nf");
    heels.setAttribute("class", kind === "unknown" ? "nf lo dash" : "nf lo");
  }

  // A frame whose numbers did not change keeps its paths.
  const B = register(stage, (dt, now) => {
    let moving = stepS(roll, dt);
    const v = {};
    for (const key of NUM) { v[key] = tval(T[key], now); if (!tdone(T[key], now)) moving = true; }
    const sig = NUM.map((key) => v[key].toFixed(3)).join() + roll.x.toFixed(2) + st.family + st.helix + st.flutes + st.guess;
    if (sig !== drawn) { drawn = sig; draw(v, rad(REST + roll.x)); }
    return moving;
  });
  bag.add(B.unregister);

  bag.add(pointer(stage, {
    move: (p) => {
      roll.t = Math.round(clamp((p[0] - 200) / 160, -1, 1) * turn);
      read.textContent = (roll.t < 0 ? "−" : "+") + Math.abs(roll.t) + "°";
      B.wake();
    },
    leave: () => { roll.t = 0; read.textContent = "rest"; B.wake(); },
  }));
  bag.add(() => svg.replaceChildren());

  return {
    set: (v) => { turn = v; },
    /** Takes a tool's description: { family, helix, flutes, diameter, fluteLength, shank, overall, tipAngle, upcut }, lengths in mm. */
    shape: (tool) => {
      st = whole(tool ?? {});
      const now = performance.now();
      for (const key of NUM) tset(T[key], st[key], now, 0);
      B.wake();
    },
    destroy: bag.dispose,
  };
}

hairline({
  name: "endmill",
  means: "A milling cutter rolls under the pointer, and takes the shape of the tool being described.",
  rules: [3, 4, 5, 8],
  range: [90, 180, 360],
  tour: [[80, 166], [320, 166], [200, 166], null],
  mount,
});
