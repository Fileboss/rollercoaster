//! Random documents built by the operations of the v0.1.0 critical path
//! (ROADMAP point 10), shared by `tests/campaign.rs` (no panic) and
//! `examples/random_cases.rs` (documents reopened by the real Looping).
//! Deterministic: a seed always gives the same sequence of operations.

use loo_format::{Document, ObjKind, Point, Rect};
use looping_core::edit::{self, Alignment, Order};
use looping_core::mcd::ObjId;

/// splitmix64: no dependency, stable across platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// 0..n (0 when n is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }

    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> Option<T> {
        (!xs.is_empty()).then(|| xs[self.below(xs.len())])
    }

    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as usize) as i32
    }
}

// names with accents, spaces, SQL reserved words and digits, to exercise
// the naming conventions
const ENTITIES: &[&str] = &[
    "Client",
    "Commande",
    "Produit",
    "Catégorie",
    "Employé",
    "Order",
    "User",
    "Ligne commande",
    "Adresse",
    "Facture",
    "Fournisseur",
    "Table",
    "Groupe",
    "Élève",
    "Cours",
    "Salle2",
    "Date",
    "Véhicule",
    "Agence",
    "Personne",
];
const ASSOCIATIONS: &[&str] =
    &["Passer", "Contenir", "Appartenir", "Livrer", "Diriger", "Suivre", "Enseigner", "Réserver", "Select", "Être né"];
const ATTRIBUTES: &[&str] = &[
    "Nom",
    "Prénom",
    "Libellé",
    "Quantité",
    "Prix",
    "date_debut",
    "Date",
    "Description",
    "Numéro",
    "key",
    "Montant TTC",
    "Âge",
    "email",
    "Statut",
    "Order",
];

pub struct Gen {
    pub rng: Rng,
    pub doc: Document,
    /// One line per operation, to reproduce a failure by hand.
    pub log: Vec<String>,
    /// Only models a designer would draw: binary or ternary associations
    /// between distinct entities (reflexive ones binary), at most one x,1
    /// leg out of binary ones, relative identification, CIF and
    /// constraints where they make sense, no inheritance cycle. Otherwise
    /// ("chaos", for the no-panic campaign) anything the editor allows.
    pub realistic: bool,
}

impl Gen {
    pub fn new(seed: u64) -> Gen {
        Gen { rng: Rng::new(seed), doc: Document::default(), log: Vec::new(), realistic: false }
    }

    /// A document after `steps` operations (the first ones always build a
    /// small model so that later operations have something to act on).
    #[allow(dead_code)] // used by examples/random_cases.rs only
    pub fn document(seed: u64, steps: usize, realistic: bool) -> Gen {
        let mut g = Gen::new(seed);
        g.realistic = realistic;
        for _ in 0..steps {
            g.step();
        }
        g
    }

    fn ids(&self, class: &str) -> Vec<ObjId> {
        self.doc.objects.iter().filter(|o| o.class == class).map(|o| o.base.id).collect()
    }

    /// First segments of the links of `class`.
    fn links(&self, class: &str) -> Vec<ObjId> {
        self.doc.objects.iter().filter(|o| o.class == class && o.base.links.prev == 0).map(|o| o.base.id).collect()
    }

    /// Legs of association `a`: (first segment, entity, cardinality code).
    fn legs(&self, a: ObjId) -> Vec<(ObjId, ObjId, i32)> {
        self.links("CDrawArc")
            .into_iter()
            .filter_map(|l| {
                let first = self.doc.object(l)?;
                let last = self.doc.object(*edit::chain_ids(&self.doc, l).last()?)?;
                (last.base.links.owner == a).then_some(())?;
                let card = self
                    .doc
                    .object(first.base.links.texts[1])
                    .and_then(|t| t.as_texte())
                    .and_then(|t| t.params.iter().find(|p| p.role == 1))
                    .map_or(2, |p| p.cardinality);
                Some((l, first.base.links.owner, card))
            })
            .collect()
    }

    /// (subtype, supertype) pairs of all inheritance links.
    fn specialisations(&self) -> Vec<(ObjId, ObjId)> {
        let mut sup = Vec::new();
        let mut sub = Vec::new();
        for l in self.links("CDrawArcHeritage") {
            let first = self.doc.object(l).unwrap();
            let Some(&last) = edit::chain_ids(&self.doc, l).last() else { continue };
            let h = self.doc.object(last).map_or(0, |o| o.base.links.owner);
            let is_sup = self
                .doc
                .object(first.base.links.texts[1])
                .and_then(|t| t.as_texte())
                .and_then(|t| t.params.iter().find(|p| p.role == 3))
                .is_some_and(|p| p.f114 != 0);
            if is_sup {
                sup.push((h, first.base.links.owner));
            } else {
                sub.push((h, first.base.links.owner));
            }
        }
        sub.iter().flat_map(|&(h, e)| sup.iter().filter(move |s| s.0 == h).map(move |s| (e, s.1))).collect()
    }

    /// Every object a user can select (no texts, no inner arc segments).
    fn selectable(&self) -> Vec<ObjId> {
        self.doc
            .objects
            .iter()
            .filter(|o| !matches!(o.kind, ObjKind::Texte(_)))
            .filter(|o| !o.class.starts_with("CDrawArc") || o.base.links.prev == 0)
            .map(|o| o.base.id)
            .collect()
    }

    fn at(&mut self) -> Point {
        Point { x: self.rng.range(0, 1200), y: self.rng.range(0, 900) }
    }

    fn name(&mut self, pool: &[&str]) -> String {
        let n = pool[self.rng.below(pool.len())];
        // sometimes a duplicate-proof suffix, sometimes the bare name
        if self.rng.chance(30) {
            format!("{n}{}", self.rng.range(1, 9))
        } else {
            n.to_string()
        }
    }

    /// An attribute of a random type, with random NOT NULL / UNIQUE.
    fn parametre(&mut self) -> loo_format::objects::Parametre {
        let name = self.name(ATTRIBUTES);
        let mut p = edit::new_parametre(&name);
        p.f20 = self.rng.range(0, 7);
        match p.f20 {
            0 => {
                p.f24 = self.rng.range(0, 2);
                p.f28 = self.rng.range(0, 2);
                p.f64 = self.rng.range(1, 255);
                p.f68 = [self.rng.range(1, 4000); 5];
            }
            1 => {
                p.f2c[0] = self.rng.range(0, 4);
                p.f2c[1] = self.rng.range(0, 3);
                p.f2c[2] = self.rng.range(0, 1);
                p.f7c = self.rng.range(1, 30);
                p.f80 = self.rng.range(0, p.f7c.min(10));
            }
            2 => {
                p.f2c[3] = self.rng.range(0, 2);
                p.f60 = self.rng.range(0, 1);
            }
            4 => p.s40 = if self.rng.chance(50) { "MONEY".into() } else { "JSON # données libres".into() },
            _ => {}
        }
        p.f88 = self.rng.chance(30) as i32;
        p.f8c = self.rng.chance(15) as i32;
        p
    }

    fn add_entity(&mut self) {
        let name = self.name(ENTITIES);
        let at = self.at();
        let e = edit::add_entity(&mut self.doc, &name, at);
        let attrs: Vec<_> = (0..self.rng.below(5)).map(|_| self.parametre()).collect();
        let shape = self.rng.below(10);
        let mut keyed: Vec<_> = (0..2).map(|_| self.parametre()).collect();
        let t = edit::texte_mut(&mut self.doc, e, 1).unwrap();
        match shape {
            // no identifier
            0 => t.params.clear(),
            // composite identifier of plain attributes
            1 => {
                t.params.clear();
                for p in &mut keyed {
                    p.f8 = 1;
                }
                t.params.extend(keyed);
            }
            _ => {}
        }
        t.params.extend(attrs);
        self.log.push(format!("entity {e} {name:?} shape {shape}"));
    }

    fn add_association(&mut self) {
        let entities = self.ids("CDrawEntite");
        if entities.is_empty() {
            return self.add_entity();
        }
        let name = self.name(ASSOCIATIONS);
        let at = self.at();
        let a = edit::add_association(&mut self.doc, &name, at);
        let mut log = format!("association {a} {name:?}");
        let legs: Vec<(ObjId, i32)> = if self.realistic {
            let mut pool = entities.clone();
            let n = if pool.len() >= 3 && self.rng.chance(20) { 3 } else { 2 };
            let mut es: Vec<ObjId> = (0..n).map(|_| pool.remove(self.rng.below(pool.len()))).collect();
            if n == 2 && self.rng.chance(10) {
                es[1] = es[0];
            }
            // a ternary has only x,n legs; a binary at most... any (1-1 included)
            es.into_iter().map(|e| (e, if n == 3 { self.rng.range(2, 3) } else { self.rng.range(0, 3) })).collect()
        } else {
            // 1 to 4 legs, a reflexive one now and then
            (0..1 + self.rng.below(4)).map(|_| (self.rng.pick(&entities).unwrap(), self.rng.range(0, 3))).collect()
        };
        for (e, c) in legs {
            let arc = edit::add_link(&mut self.doc, e, a, c);
            log += &format!(" leg {arc}:{e}/{c}");
        }
        if self.rng.chance(30) {
            let p = self.parametre();
            edit::texte_mut(&mut self.doc, a, 1).unwrap().params.push(p);
        }
        self.log.push(log);
    }

    /// Relative identification on a 1,1 leg.
    fn relative(&mut self) {
        let legs: Vec<ObjId> = self
            .links("CDrawArc")
            .into_iter()
            .filter(|&l| {
                edit::texte_mut(&mut self.doc, l, 1)
                    .is_some_and(|t| t.params.iter().any(|p| p.role == 1 && p.cardinality == 1))
            })
            .collect();
        // a 1,1 leg of a binary association to another entity
        let legs: Vec<ObjId> = if self.realistic {
            let assos = self.ids("CDrawRelation");
            legs.into_iter()
                .filter(|&l| {
                    assos.iter().any(|&a| {
                        let ls = self.legs(a);
                        ls.len() == 2 && ls.iter().any(|x| x.0 == l) && ls[0].1 != ls[1].1
                    })
                })
                .collect()
        } else {
            legs
        };
        if let Some(l) = self.rng.pick(&legs) {
            let t = edit::texte_mut(&mut self.doc, l, 1).unwrap();
            t.params.iter_mut().find(|p| p.role == 1).unwrap().f84 = 1;
            self.log.push(format!("relative {l}"));
        }
    }

    fn role(&mut self) {
        let legs = self.links("CDrawArc");
        if let Some(l) = self.rng.pick(&legs) {
            let mut p = edit::new_parametre("");
            p.role = 2;
            p.sf0 = self.name(&["chef", "membre", "parent", "enfant", "auteur"]);
            p.n130 = self.rng.range(0, 1);
            p.f134 = self.rng.range(0, 1);
            let t = edit::texte_mut(&mut self.doc, l, 1).unwrap();
            t.params.retain(|p| p.role != 2);
            t.params.push(p);
            self.log.push(format!("role {l}"));
        }
    }

    fn heritage(&mut self) {
        let entities = self.ids("CDrawEntite");
        if entities.len() < 2 {
            return self.add_entity();
        }
        let at = self.at();
        let constraint = self.rng.range(0, 3);
        let h = edit::add_heritage(&mut self.doc, at, constraint);
        if let Some(ObjKind::Heritage(x)) = self.doc.objects.iter_mut().find(|o| o.base.id == h).map(|o| &mut o.kind) {
            x.f418 = self.rng.range(0, 1);
        }
        let sup = self.rng.pick(&entities).unwrap();
        edit::add_heritage_link(&mut self.doc, sup, h, true);
        // realistic: a subtype has one supertype, and is not above `sup`
        let specs = self.specialisations();
        let mut above = vec![sup];
        while let Some(&(_, s)) = specs.iter().find(|(e, s)| above.contains(e) && !above.contains(s)) {
            above.push(s);
        }
        let candidates: Vec<ObjId> = if self.realistic {
            entities.iter().copied().filter(|e| !above.contains(e) && !specs.iter().any(|x| x.0 == *e)).collect()
        } else {
            entities.clone()
        };
        let mut subs = Vec::new();
        for _ in 0..1 + self.rng.below(3) {
            let Some(e) = self.rng.pick(&candidates) else { break };
            if e != sup && !subs.contains(&e) {
                edit::add_heritage_link(&mut self.doc, e, h, false);
                subs.push(e);
            }
        }
        self.log.push(format!("heritage {h} constraint {constraint} sup {sup} subs {subs:?}"));
    }

    /// CIF on an association: the association, a targeted entity and
    /// some emitters among its entities.
    fn cif(&mut self) {
        let mut assos = self.ids("CDrawRelation");
        if self.realistic {
            assos.retain(|&a| {
                let ls = self.legs(a);
                ls.len() >= 3 && ls.iter().all(|l| l.2 >= 2)
            });
        }
        let Some(a) = self.rng.pick(&assos) else { return };
        let ends: Vec<ObjId> = self
            .links("CDrawArc")
            .into_iter()
            .filter_map(|l| {
                let chain = edit::chain_ids(&self.doc, l);
                let last = self.doc.object(*chain.last()?)?.base.links.owner;
                (last == a).then(|| self.doc.object(l).unwrap().base.links.owner)
            })
            .collect();
        if ends.len() < 2 {
            return;
        }
        let at = self.at();
        let c = edit::add_cif(&mut self.doc, at);
        edit::add_cif_link(&mut self.doc, a, c, false);
        let target = self.rng.pick(&ends).unwrap();
        edit::add_cif_link(&mut self.doc, target, c, true);
        let mut emitters = Vec::new();
        for &e in &ends {
            if e != target && !emitters.contains(&e) && self.rng.chance(60) {
                edit::add_cif_link(&mut self.doc, e, c, false);
                emitters.push(e);
            }
        }
        self.log.push(format!("cif {c} on {a} target {target} emitters {emitters:?}"));
    }

    fn contrainte(&mut self) {
        let assos = self.ids("CDrawRelation");
        if assos.len() < 2 {
            return;
        }
        let at = self.at();
        let kind = self.rng.range(0, 4);
        let name = ["X", "T", "XT", "S", "I"][kind as usize];
        let k = edit::add_contrainte(&mut self.doc, name, at, kind);
        let a = self.rng.pick(&assos).unwrap();
        let mut b = self.rng.pick(&assos).unwrap();
        if a == b {
            b = assos[(assos.iter().position(|&x| x == a).unwrap() + 1) % assos.len()];
        }
        let la = edit::add_contrainte_link(&mut self.doc, a, k);
        edit::add_contrainte_link(&mut self.doc, b, k);
        if self.realistic {
            let ea: Vec<ObjId> = self.legs(a).iter().map(|l| l.1).collect();
            if let Some(&(_, pivot, _)) = self.legs(b).iter().find(|l| ea.contains(&l.1)) {
                edit::add_contrainte_link(&mut self.doc, pivot, k);
            }
            if kind == 4 {
                // the first association is the target
                let mut p = edit::new_parametre("");
                p.role = 4;
                p.f114 = 1;
                edit::texte_mut(&mut self.doc, la, 1).unwrap().params.push(p);
            }
        }
        if self.rng.chance(30) {
            edit::set_script(&mut self.doc, k, &["-- contrainte"], 0);
        }
        self.log.push(format!("contrainte {k} kind {kind} on {a} {b}"));
    }

    fn regle(&mut self) {
        let at = self.at();
        let after = self.rng.chance(50);
        let flags = [self.rng.range(0, 1), self.rng.range(0, 1), self.rng.range(0, 1)];
        let r = edit::add_regle(
            &mut self.doc,
            "R",
            at,
            &["Une règle", "sur deux lignes"],
            &["UPDATE Client SET Nom = 'x';"],
            after,
            flags,
        );
        let targets: Vec<ObjId> = self.ids("CDrawEntite").into_iter().chain(self.ids("CDrawRelation")).collect();
        if let Some(t) = self.rng.pick(&targets) {
            edit::add_regle_link(&mut self.doc, t, r);
        }
        self.log.push(format!("regle {r} after {after} flags {flags:?}"));
    }

    fn set_cardinality(&mut self) {
        if let Some(l) = self.rng.pick(&self.links("CDrawArc")) {
            let n_ary = self.ids("CDrawRelation").into_iter().any(|a| {
                let ls = self.legs(a);
                ls.len() > 2 && ls.iter().any(|x| x.0 == l)
            });
            let c = if self.realistic && n_ary { self.rng.range(2, 3) } else { self.rng.range(0, 3) };
            edit::set_cardinality(&mut self.doc, l, c);
            self.log.push(format!("cardinality {l} {c}"));
        }
    }

    fn rename(&mut self) {
        let targets: Vec<ObjId> = self.ids("CDrawEntite").into_iter().chain(self.ids("CDrawRelation")).collect();
        if let Some(t) = self.rng.pick(&targets) {
            let pool = if self.ids("CDrawEntite").contains(&t) { ENTITIES } else { ASSOCIATIONS };
            let name = self.name(pool);
            edit::rename(&mut self.doc, t, &name);
            self.log.push(format!("rename {t} {name:?}"));
        }
    }

    fn delete(&mut self) {
        if let Some(t) = self.rng.pick(&self.selectable()) {
            edit::delete(&mut self.doc, t);
            self.log.push(format!("delete {t}"));
        }
    }

    fn geometry(&mut self) {
        let all = self.selectable();
        let Some(t) = self.rng.pick(&all) else { return };
        let some: Vec<ObjId> = all.iter().copied().filter(|_| self.rng.chance(30)).collect();
        match self.rng.below(9) {
            0 => {
                let (dx, dy) = (self.rng.range(-200, 200), self.rng.range(-200, 200));
                edit::translate(&mut self.doc, t, dx, dy);
                self.log.push(format!("translate {t} {dx} {dy}"));
            }
            1 => {
                let at = self.at();
                let r = Rect {
                    left: at.x,
                    top: at.y,
                    right: at.x + self.rng.range(1, 400),
                    bottom: at.y + self.rng.range(1, 300),
                };
                edit::resize(&mut self.doc, t, r);
                self.log.push(format!("resize {t} {r:?}"));
            }
            2 => {
                if let Some(l) = self.rng.pick(&self.links("CDrawArc")) {
                    let at = self.at();
                    let seg = self.rng.pick(&edit::chain_ids(&self.doc, l)).unwrap();
                    edit::add_break(&mut self.doc, seg, at);
                    self.log.push(format!("add_break {seg}"));
                }
            }
            3 => {
                if let Some(l) = self.rng.pick(&self.links("CDrawArc")) {
                    let seg = self.rng.pick(&edit::chain_ids(&self.doc, l)).unwrap();
                    edit::remove_break(&mut self.doc, seg);
                    self.log.push(format!("remove_break {seg}"));
                }
            }
            4 => {
                let clip = edit::copy(&self.doc, &some);
                let new = edit::paste(&mut self.doc, &clip, 40, 40);
                self.log.push(format!("copy/paste {some:?} -> {new:?}"));
            }
            5 => {
                let o = [Order::Front, Order::Back, Order::Forward, Order::Backward][self.rng.below(4)];
                edit::reorder(&mut self.doc, &some, o);
                self.log.push(format!("reorder {some:?}"));
            }
            6 => {
                let a = [
                    Alignment::Left,
                    Alignment::Centre,
                    Alignment::Right,
                    Alignment::Top,
                    Alignment::Middle,
                    Alignment::Bottom,
                ][self.rng.below(6)];
                edit::align(&mut self.doc, &some, a);
                self.log.push(format!("align {some:?}"));
            }
            7 => {
                let step = self.rng.range(1, 50);
                edit::snap(&mut self.doc, t, step);
                self.log.push(format!("snap {t} {step}"));
            }
            _ => {
                edit::fit_all(&mut self.doc);
                self.log.push("fit_all".into());
            }
        }
    }

    fn views(&mut self) {
        match self.rng.below(3) {
            0 => {
                let bit = edit::add_view(&mut self.doc, "Vue");
                self.log.push(format!("add_view {bit:?}"));
            }
            1 => {
                if let Some(t) = self.rng.pick(&self.selectable()) {
                    let mask = 1u32 << self.rng.range(1, 20);
                    let member = self.rng.chance(70);
                    edit::set_views(&mut self.doc, t, mask, member);
                    self.log.push(format!("set_views {t} {mask:#x} {member}"));
                }
            }
            _ => {
                let bit = self.rng.range(1, 20);
                edit::remove_view(&mut self.doc, bit);
                self.log.push(format!("remove_view {bit}"));
            }
        }
    }

    /// One of the options of the "Options SQL" menu.
    fn option(&mut self) {
        let (off, max) =
            [(0x52c, 8), (0xa78, 5), (0xab8, 1), (0x7f0, 1), (0x7f4, 2), (0xb40, 2), (0x7f8, 3), (0x888, 4)]
                [self.rng.below(8)];
        let v = self.rng.range(0, max);
        self.doc.settings.set_i32(off, v);
        self.log.push(format!("option {off:#x} = {v}"));
    }

    /// One random operation, weighted towards building the model.
    pub fn step(&mut self) {
        if self.ids("CDrawEntite").len() < 2 {
            return self.add_entity();
        }
        match self.rng.below(100) {
            0..=17 => self.add_entity(),
            18..=37 => self.add_association(),
            38..=42 => self.relative(),
            43..=46 => self.role(),
            47..=51 => self.heritage(),
            52..=55 => self.cif(),
            56..=58 => self.contrainte(),
            59..=61 => self.regle(),
            62..=67 => self.set_cardinality(),
            68..=71 => self.rename(),
            72..=76 => self.delete(),
            77..=89 => self.geometry(),
            90..=92 => self.views(),
            _ => self.option(),
        }
    }
}
