(() => {
  var __defProp = Object.defineProperty;
  var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

  // src/log-parser.ts
  function resolveChatChannel(category) {
    if (category === "Proximit\xE9") return { key: "proximite", label: "Proximit\xE9" };
    if (category === "Guilde") return { key: "guilde", label: "Guilde" };
    if (category === "Commerce") return { key: "commerce", label: "Commerce" };
    if (category === "Groupe" || category === "\xC9quipe") {
      return { key: "groupe", label: "Groupe" };
    }
    if (category.startsWith("Recrutement")) {
      return { key: "recrutement", label: "Recrutement" };
    }
    if (category.startsWith("Communaut\xE9")) {
      return { key: "communaute", label: "Communaut\xE9" };
    }
    return null;
  }
  function parseFrenchNumber(raw) {
    return parseInt(raw.replace(/\D/g, ""), 10);
  }
  var NUM = "[\\d \\u00A0\\u202F]+";
  var HEADER_RE = /^\s*(INFO|WARN|ERROR)\s+(\d{2}:\d{2}:\d{2},\d{3})\s+\[[^\]]*\]\s+\([^)]*\)\s+-\s+(.*)$/;
  var BRACKET_RE = /^\[([^\]]+)\] ?(.*)$/;
  var CHAT_CONTENT_RE = /^(.+?) : (.*)$/;
  var KAMA_GAIN_RE = new RegExp(`^Vous avez gagn\xE9 (${NUM}) kamas\\.?$`);
  var KAMA_LOSS_RE = new RegExp(`^Vous avez perdu (${NUM}) kamas\\.?$`);
  var RAMASSE_RE = new RegExp(`^Vous avez ramass\xE9 (${NUM})x (.+?)\\s*\\.?$`);
  var CHALLENGE_SUCCESS_RE = /^Le challenge "(.+?)" est réussi\.?$/;
  var CHALLENGE_FAIL_RE = /^Le challenge "(.+?)" a échoué\.?$/;
  var XP_RE = new RegExp(`^(.+?) : \\+(${NUM}) points d'XP\\.`);
  var SPELL_CAST_RE = /^(.+?) lance le sort (.+)$/;
  var CRITICAL_SUFFIX_RE = /^(.*) \(Critiques\)$/;
  var KO_RE = /^(.+?) est KO !$/;
  var HORS_COMBAT_RE = /^(.+?) est hors-combat !$/;
  var DISAPPEAR_RE = /^(.+?): disparaît$/;
  var SUMMON_ANNOUNCE_RE = /^(.+?): Invoque (?:un\(e\) (.+?)|une créature du .+?)\s*$/;
  var SUMMON_JOIN_WINDOW_MS = 500;
  var INVOCATION_INSTANTIATED_RE = /^(?:Instanciation d'une nouvelle invocation avec un id de|New summon with id) -?\d+$/;
  var ANNOUNCE_TO_INSTANTIATION_WINDOW_MS = 50;
  var TRANSFORM_RE = /^(.+?): transformée? en (.+?)\s*!?$/;
  var DEFEAT_MARKER_RE = /^Vous avez été vaincu\(e\) !$/;
  var OCCUPATION_RE = /^Lancement de l'occupation pour le joueur (.+)$/;
  var FIGHT_END_RE = /^\[FIGHT\] End fight with id (-?\d+)$/;
  var COMBAT_START_MARKER = "CREATION DU COMBAT";
  var MARKET_OCCUPATION_START_RE = /^Lancement de l'occupation MARKET sur la board\b/;
  var MARKET_OCCUPATION_END_RE = /^On arrête l'occupation MARKET sur la board\b/;
  var CLIENT_BUILD_DATE_RE = /\[(\d{4})-(\d{2})-(\d{2}) @ (\d{2})H(\d{2})min(\d{2})\]/;
  var FIGHTER_JOIN_RE = /^fightId=(-?\d+) (.+?) breed : (\d+) \[(-?\d+)\] isControlledByAI=(true|false) obstacleId : (-?\d+) join the fight/;
  var DAMAGE_RE = new RegExp(`^(.+?): ([+-])(${NUM}) PV\\b(.*)$`);
  var ARMOR_RE = new RegExp(`^(.+?): ([+-]?)(${NUM}) Armure\\b(.*)$`);
  var TAG_RE = /\(([^)]+)\)/g;
  var STATUS_EFFECT_RE = new RegExp(`^(.+?): (.+?) \\((?:Niv\\. ${NUM}|\\+${NUM} Niv\\.)\\)$`);
  var STATUS_REMOVE_RE = /^(.+?): n'est plus sous l'emprise de '(.+?)'\.?$/;
  var IGNORED_TAG = "Parade !";
  var TRADE_DONNE_RE = /le joueur (.+?) donne\s*:\s*(\d+)\s*K\s*;\s*(.*?)(?=le joueur .+? donne\s*:|$)/g;
  var TRADE_ITEM_RE = /(\d+)\s*x\s*(.+?)\s*\(refId=-?\d+\)/g;
  var DAMAGE_ELEMENTS = /* @__PURE__ */ new Set([
    "Neutre",
    "Terre",
    "Feu",
    "Eau",
    "Air",
    "Lumi\xE8re",
    "Stasis"
  ]);
  var DEDUPE_WINDOW_MS = 1e3;
  var DEDUPE_EXEMPT_KINDS = /* @__PURE__ */ new Set(["loot", "fighter-joined"]);
  function createFightParseState() {
    return {
      lastCast: null,
      lastDamage: null,
      effectOwners: /* @__PURE__ */ new Map(),
      spellCasters: /* @__PURE__ */ new Map(),
      summonOwners: /* @__PURE__ */ new Map(),
      pendingSummonCasters: [],
      seenFighterIds: /* @__PURE__ */ new Set()
    };
  }
  var LogParser = class {
    /**
     * `isKnownMonsterName` : prédicat optionnel injecté par l'appelant (voir StatsStoreService, seule
     * couche qui a accès au catalogue — LogParser reste volontairement pur/sans dépendance réseau ou
     * catalogue) — voir parseFighterJoin pour son unique usage : distinguer un vrai monstre catalogué
     * d'un nom d'invocation inconnu quand `INVOCATION_INSTANTIATED_RE` (repli SANS annonce "Invoque")
     * est sur le point de l'avaler à tort. Par défaut (tests, `new LogParser()` sans argument) toujours
     * `false` — comportement historique inchangé, aucune régression sur les ~200 tests existants qui ne
     * couvrent pas ce cas précis.
     */
    constructor(deps = {}) {
      __publicField(this, "deps", deps);
      /** Un état d'attribution par combat actif (voir FightParseState), clé `null` = hors combat/non résolu. */
      __publicField(this, "fightStates", /* @__PURE__ */ new Map());
      /** Combats connus pour un nom de combattant donné — un nom peut appartenir à plusieurs combats concurrents si le même monstre apparaît dans deux combats simultanés (multi-compte). */
      __publicField(this, "nameToFightIds", /* @__PURE__ */ new Map());
      /** Inverse de nameToFightIds, pour retirer proprement un combat terminé (évite qu'un nom de monstre très courant reste ambigu pour de futurs combats sans rapport). */
      __publicField(this, "fightMemberNames", /* @__PURE__ */ new Map());
      __publicField(this, "fightLostFlags", /* @__PURE__ */ new Map());
      /** Dernier combat résolu sans ambiguïté : repli pour les lignes sans nom exploitable (tour, butin) ou dont le nom est ambigu. */
      __publicField(this, "currentFightId", null);
      /** Ligne en cours d'accumulation : un enregistrement Java peut s'étaler sur plusieurs lignes physiques (ex. résumé d'échange), la suite n'ayant pas d'en-tête LEVEL/horodatage. */
      __publicField(this, "pending", null);
      /** Horodatage (ms depuis minuit) de la dernière occurrence de chaque signature d'événement, pour ignorer les doublons multi-compte. */
      __publicField(this, "recentSignatures", /* @__PURE__ */ new Map());
    }
    parseLine(rawLine) {
      const line = rawLine.replace(/\r$/, "");
      if (!line.trim()) return null;
      const headerMatch = HEADER_RE.exec(line);
      if (headerMatch) {
        const flushed = this.flushPending();
        const [, level, time, firstPart] = headerMatch;
        this.pending = level === "INFO" ? { time, parts: [firstPart] } : null;
        return flushed;
      }
      this.pending?.parts.push(line.trim());
      return null;
    }
    /**
     * Un enregistrement peut s'étaler sur plusieurs lignes physiques (voir
     * parseLine) : celui de la toute dernière ligne d'un lot n'est donc traité
     * qu'à la ligne suivante, pour savoir s'il continue. À appeler après avoir
     * traité un lot complet de lignes, pour ne pas laisser la dernière en
     * attente indéfiniment si aucune nouvelle ligne n'arrive avant longtemps.
     */
    flush() {
      return this.flushPending();
    }
    /** Réinitialise tout l'état interne (à appeler à chaque reconnexion/relecture complète du fichier). */
    reset() {
      this.fightStates.clear();
      this.nameToFightIds.clear();
      this.fightMemberNames.clear();
      this.fightLostFlags.clear();
      this.currentFightId = null;
      this.pending = null;
      this.recentSignatures.clear();
    }
    flushPending() {
      if (!this.pending) return null;
      const { time, parts } = this.pending;
      this.pending = null;
      const content = parts.join(" ").trim();
      if (!content) return null;
      const entry = this.parseContent(time, content);
      if (!entry) return null;
      return this.isDuplicate(entry) ? null : entry;
    }
    parseContent(time, content) {
      const fightEnd = FIGHT_END_RE.exec(content);
      if (fightEnd) {
        const fightId = Number(fightEnd[1]);
        const lost = this.fightLostFlags.get(fightId) ?? false;
        this.forgetFight(fightId);
        return { kind: "combat-end", time, fightId, result: lost ? "lost" : "won" };
      }
      if (content === COMBAT_START_MARKER) {
        return { kind: "combat-start", time };
      }
      if (MARKET_OCCUPATION_START_RE.test(content)) {
        return { kind: "market-occupation", time, active: true };
      }
      if (MARKET_OCCUPATION_END_RE.test(content)) {
        return { kind: "market-occupation", time, active: false };
      }
      if (INVOCATION_INSTANTIATED_RE.test(content)) {
        const state = this.getFightState(this.resolveCurrentFightId());
        const nowMs = this.timeToMs(time);
        const lastPending = state.pendingSummonCasters[state.pendingSummonCasters.length - 1];
        const justAnnounced = !!lastPending && nowMs - lastPending.timeMs <= ANNOUNCE_TO_INSTANTIATION_WINDOW_MS;
        if (!justAnnounced && state.lastCast) {
          state.pendingSummonCasters.push({
            caster: state.lastCast.caster,
            timeMs: nowMs,
            source: "fallback",
            expectedName: null
          });
        }
        return null;
      }
      const buildDate = CLIENT_BUILD_DATE_RE.exec(content);
      if (buildDate) {
        const [, year, month, day] = buildDate;
        return {
          kind: "log-date-anchor",
          time,
          year: Number(year),
          month: Number(month),
          day: Number(day)
        };
      }
      const bracketMatch = BRACKET_RE.exec(content);
      if (!bracketMatch) return null;
      const [, category, rest] = bracketMatch;
      const bracketContent = (rest ?? "").trim();
      const chatChannel = resolveChatChannel(category);
      if (chatChannel) {
        const chatMatch = CHAT_CONTENT_RE.exec(bracketContent);
        if (!chatMatch) return null;
        return {
          kind: "chat",
          time,
          channel: chatChannel.key,
          channelLabel: chatChannel.label,
          author: chatMatch[1].trim(),
          message: chatMatch[2].trim()
        };
      }
      if (category === "Information (jeu)") {
        return this.parseGameLine(time, bracketContent);
      }
      if (category === "Information (combat)") {
        return this.parseCombatLine(time, bracketContent);
      }
      if (category === "_FL_") {
        return this.parseFighterJoin(time, bracketContent);
      }
      if (category === "DEATH") {
        const occupation = OCCUPATION_RE.exec(bracketContent);
        if (occupation) {
          const fightId = this.resolveFightIdForOccupation(occupation[1].trim());
          if (fightId !== null) this.fightLostFlags.set(fightId, true);
          return { kind: "combat-defeat-marker", time, fightId };
        }
        return null;
      }
      if (category === "Trade") {
        return this.parseTradeLine(bracketContent, time);
      }
      return null;
    }
    parseTradeLine(content, time) {
      const sides = [];
      TRADE_DONNE_RE.lastIndex = 0;
      for (const match of content.matchAll(TRADE_DONNE_RE)) {
        const playerName = match[1].trim();
        const kamas = Number(match[2]);
        const itemsText = match[3];
        const items = [];
        for (const itemMatch of itemsText.matchAll(TRADE_ITEM_RE)) {
          items.push({ quantity: Number(itemMatch[1]), name: itemMatch[2].trim() });
        }
        sides.push({ playerName, items, kamas });
      }
      if (sides.length !== 2) return null;
      return { kind: "trade-completed", time, sides: [sides[0], sides[1]] };
    }
    parseFighterJoin(time, content) {
      const join = FIGHTER_JOIN_RE.exec(content);
      if (!join) return null;
      const fightId = Number(join[1]);
      const name = join[2].trim();
      const breed = Number(join[3]);
      const fighterId = Number(join[4]);
      const isControlledByAI = join[5] === "true";
      const state = this.getFightState(fightId);
      const isNewFighter = !state.seenFighterIds.has(fighterId);
      state.seenFighterIds.add(fighterId);
      const joinTimeMs = this.timeToMs(time);
      while (state.pendingSummonCasters.length > 0 && joinTimeMs - state.pendingSummonCasters[0].timeMs > SUMMON_JOIN_WINDOW_MS) {
        state.pendingSummonCasters.shift();
      }
      let summonedBy = state.summonOwners.get(name) ?? null;
      if (!summonedBy && isNewFighter && state.pendingSummonCasters.length > 0) {
        const matchIndex = state.pendingSummonCasters.findIndex(
          (p) => p.expectedName === null || p.expectedName.toLowerCase() === name.toLowerCase()
        );
        if (matchIndex !== -1) {
          const [pending] = state.pendingSummonCasters.splice(matchIndex, 1);
          const isUnreliableFallbackOnRealMonster = pending.source === "fallback" && (this.deps.isKnownMonsterName?.(name) ?? false);
          if (!isUnreliableFallbackOnRealMonster) {
            summonedBy = pending.caster;
            state.summonOwners.set(name, summonedBy);
          }
        }
      }
      let fightIds = this.nameToFightIds.get(name);
      if (!fightIds) {
        fightIds = /* @__PURE__ */ new Set();
        this.nameToFightIds.set(name, fightIds);
      }
      fightIds.add(fightId);
      let members = this.fightMemberNames.get(fightId);
      if (!members) {
        members = /* @__PURE__ */ new Set();
        this.fightMemberNames.set(fightId, members);
      }
      members.add(name);
      this.currentFightId = fightId;
      return {
        kind: "fighter-joined",
        time,
        fightId,
        name,
        breed,
        fighterId,
        isControlledByAI,
        summonedBy
      };
    }
    /** Oublie un combat terminé : libère les noms de combattants qui n'appartiennent à aucun autre combat actif, pour éviter qu'un nom de monstre courant reste faussement ambigu pour un futur combat sans rapport. */
    forgetFight(fightId) {
      const members = this.fightMemberNames.get(fightId);
      if (members) {
        for (const name of members) {
          const ids = this.nameToFightIds.get(name);
          if (!ids) continue;
          ids.delete(fightId);
          if (ids.size === 0) this.nameToFightIds.delete(name);
        }
      }
      this.fightMemberNames.delete(fightId);
      this.fightLostFlags.delete(fightId);
      this.fightStates.delete(fightId);
      if (this.currentFightId === fightId) this.currentFightId = null;
    }
    /** État d'attribution (voir FightParseState) du combat `fightId`, créé au premier accès. */
    getFightState(fightId) {
      let state = this.fightStates.get(fightId);
      if (!state) {
        state = createFightParseState();
        this.fightStates.set(fightId, state);
      }
      return state;
    }
    /** Résout le combat d'un combattant nommé : sans ambiguïté si ce nom n'appartient qu'à un seul combat actif, sinon repli sur le dernier combat résolu (voir resolveCurrentFightId). */
    resolveFightIdForName(name) {
      const ids = this.nameToFightIds.get(name);
      if (ids && ids.size === 1) {
        const [id] = ids;
        this.currentFightId = id;
        return id;
      }
      return this.resolveCurrentFightId();
    }
    /** "Lancement de l'occupation pour le joueur {nom} {classe}" : le nom du combattant est un préfixe du texte capturé (la classe suit, ex. "Crâ", "Sram"). */
    resolveFightIdForOccupation(rawName) {
      let resolved = null;
      let ambiguous = false;
      for (const [name, ids] of this.nameToFightIds) {
        if (rawName !== name && !rawName.startsWith(`${name} `)) continue;
        for (const id of ids) {
          if (resolved !== null && resolved !== id) ambiguous = true;
          resolved = id;
        }
      }
      if (ambiguous) return this.resolveCurrentFightId();
      return resolved ?? this.resolveCurrentFightId();
    }
    /**
     * Repli utilisé pour les lignes sans nom de combattant exploitable (butin,
     * changement de tour, marqueur de défaite "vaincu(e)") : le dernier combat
     * résolu sans ambiguïté, s'il est toujours actif — sinon, s'il ne reste
     * plus qu'UN SEUL combat actif, ce dernier ne peut être que le bon (plus
     * d'ambiguïté possible). Sans ce second repli, la fin d'un premier combat
     * concurrent laissait `currentFightId` à `null` jusqu'à la prochaine ligne
     * à nom résolvable, et tout butin ramassé entre-temps pour l'unique combat
     * restant se perdait (bug réel : butin de fin de combat manquant en
     * multi-compte, voir tests).
     */
    resolveCurrentFightId() {
      if (this.currentFightId !== null && this.fightMemberNames.has(this.currentFightId)) {
        return this.currentFightId;
      }
      if (this.fightMemberNames.size === 1) {
        const [onlyId] = this.fightMemberNames.keys();
        this.currentFightId = onlyId;
        return onlyId;
      }
      return null;
    }
    parseGameLine(time, content) {
      const gain = KAMA_GAIN_RE.exec(content);
      if (gain) {
        return {
          kind: "kama-gain",
          time,
          amount: parseFrenchNumber(gain[1]),
          fightId: this.resolveCurrentFightId()
        };
      }
      const loss = KAMA_LOSS_RE.exec(content);
      if (loss) {
        return { kind: "kama-loss", time, amount: parseFrenchNumber(loss[1]) };
      }
      const loot = RAMASSE_RE.exec(content);
      if (loot) {
        return {
          kind: "loot",
          time,
          item: loot[2].trim(),
          quantity: parseFrenchNumber(loot[1]),
          fightId: this.resolveCurrentFightId()
        };
      }
      const challengeSuccess = CHALLENGE_SUCCESS_RE.exec(content);
      if (challengeSuccess) {
        return {
          kind: "challenge-result",
          time,
          name: challengeSuccess[1].trim(),
          success: true,
          fightId: this.resolveCurrentFightId()
        };
      }
      const challengeFail = CHALLENGE_FAIL_RE.exec(content);
      if (challengeFail) {
        return {
          kind: "challenge-result",
          time,
          name: challengeFail[1].trim(),
          success: false,
          fightId: this.resolveCurrentFightId()
        };
      }
      return null;
    }
    parseCombatLine(time, content) {
      if (DEFEAT_MARKER_RE.test(content)) {
        return { kind: "combat-defeat-marker", time, fightId: this.resolveCurrentFightId() };
      }
      const ko = KO_RE.exec(content);
      if (ko) {
        const name = ko[1].trim();
        return { kind: "enemy-defeated", time, name, fightId: this.resolveFightIdForName(name) };
      }
      const horsCombat = HORS_COMBAT_RE.exec(content);
      if (horsCombat) {
        const name = horsCombat[1].trim();
        return { kind: "enemy-defeated", time, name, fightId: this.resolveFightIdForName(name) };
      }
      const disappear = DISAPPEAR_RE.exec(content);
      if (disappear) {
        const name = disappear[1].trim();
        return { kind: "enemy-fled", time, name, fightId: this.resolveFightIdForName(name) };
      }
      const summonAnnounce = SUMMON_ANNOUNCE_RE.exec(content);
      if (summonAnnounce) {
        const caster = summonAnnounce[1].trim();
        const expectedName = summonAnnounce[2]?.trim() ?? null;
        const fightId = this.resolveFightIdForName(caster);
        this.getFightState(fightId).pendingSummonCasters.push({
          caster,
          timeMs: this.timeToMs(time),
          source: "announce",
          expectedName
        });
        return null;
      }
      const transform = TRANSFORM_RE.exec(content);
      if (transform) {
        const from = transform[1].trim();
        const to = transform[2].trim();
        const state = this.getFightState(this.resolveFightIdForName(from));
        const owner = this.resolveSummonRootOwner(state, from);
        if (owner) state.summonOwners.set(to, owner);
        return null;
      }
      const cast = SPELL_CAST_RE.exec(content);
      if (cast) {
        const caster = cast[1].trim();
        let spell = cast[2].trim();
        let critical = false;
        const critMatch = CRITICAL_SUFFIX_RE.exec(spell);
        if (critMatch) {
          spell = critMatch[1].trim();
          critical = true;
        }
        const fightId = this.resolveFightIdForName(caster);
        const state = this.getFightState(fightId);
        state.lastCast = { caster, spell };
        state.spellCasters.set(spell.toLowerCase(), caster);
        return { kind: "spell-cast", time, caster, spell, critical, fightId };
      }
      const statusRemoval = STATUS_REMOVE_RE.exec(content);
      if (statusRemoval) {
        const carrier = statusRemoval[1].trim();
        const fightId = this.resolveFightIdForName(carrier);
        this.getFightState(fightId).effectOwners.delete(statusRemoval[2].trim().toLowerCase());
        return null;
      }
      const statusEffect = STATUS_EFFECT_RE.exec(content);
      if (statusEffect) {
        const carrier = statusEffect[1].trim();
        const effectName = statusEffect[2].trim();
        const state = this.getFightState(this.resolveFightIdForName(carrier));
        state.effectOwners.set(effectName.toLowerCase(), {
          carrier,
          applier: state.lastCast?.caster ?? carrier
        });
        return null;
      }
      const xp = XP_RE.exec(content);
      if (xp) {
        const character = xp[1].trim();
        return {
          kind: "xp-gain",
          time,
          character,
          amount: parseFrenchNumber(xp[2]),
          fightId: this.resolveFightIdForName(character)
        };
      }
      const damage = DAMAGE_RE.exec(content);
      if (damage) {
        const sign = damage[2];
        const target = damage[1].trim();
        const amount = parseFrenchNumber(damage[3]);
        const tail = damage[4] ?? "";
        const fightId = this.resolveFightIdForName(target);
        const state = this.getFightState(fightId);
        if (sign === "-") {
          const { attacker: attacker2, spell: spell2, element: element2 } = this.resolveEffectTail(target, tail, state, {
            selfFallback: false,
            riposteFallback: true
          });
          state.lastDamage = { attacker: attacker2, target };
          return { kind: "damage", time, target, attacker: attacker2, spell: spell2, element: element2, amount, fightId };
        }
        const { attacker, spell, element } = this.resolveEffectTail(target, tail, state, {
          selfFallback: true,
          riposteFallback: false
        });
        return { kind: "heal", time, target, attacker, spell, element, amount, fightId };
      }
      const armor = ARMOR_RE.exec(content);
      if (armor) {
        const sign = armor[2];
        if (sign === "-") return null;
        const target = armor[1].trim();
        const amount = parseFrenchNumber(armor[3]);
        const tail = armor[4] ?? "";
        const fightId = this.resolveFightIdForName(target);
        const { attacker, spell } = this.resolveEffectTail(
          target,
          tail,
          this.getFightState(fightId),
          {
            selfFallback: true,
            riposteFallback: false
          }
        );
        return { kind: "armor", time, target, attacker, spell, amount, fightId };
      }
      return null;
    }
    /**
     * Résout qui créditer (`attacker`) et le nom de la source (`spell`) d'une ligne PV/Armure à
     * partir de son tag de fin de ligne (parenthèses) — mécanique commune aux dégâts, soins et
     * armure donnée (voir CLAUDE.md). Un tag reconnu comme élément (voir DAMAGE_ELEMENTS) alimente
     * `element` ; le dernier tag NON élémentaire (hors "Parade !", jamais une cause) est le tag
     * "mécanique" (statut, glyphe, riposte...) qui détermine `attacker`/`spell`.
     *
     * - `riposteFallback: true` (dégâts uniquement, comportement historique inchangé) : à défaut de
     *   statut suivi (`effectOwners`) ou de sort connu (`spellCasters`, ex. glyphe "Canine" posé une
     *   fois qui tape bien plus tard), une riposte pure (ex. "Contre-attaque") crédite la victime du
     *   coup précédent.
     * - `riposteFallback: false` (soins/armure) : un tag non suivi par `effectOwners` est un passif
     *   propre à la cible (ex. "Art Canin", "Digestion") — crédité à la cible elle-même plutôt qu'au
     *   dernier sort connu, qui pourrait être sans rapport (lancé par un tiers). `spellCasters` n'est
     *   volontairement PAS consulté ici (contrairement aux dégâts) : il est global et persiste au-delà
     *   du tour pour N'IMPORTE QUEL sort déjà lancé par N'IMPORTE QUI, ce qui créditerait à tort un
     *   adversaire pour le passif défensif propre de sa cible (ex. armure gagnée par la cible d'une
     *   attaque, taguée du nom du sort qui vient de la toucher — cas réel constaté, voir tests).
     *
     * Dernière étape, commune à tous les appelants : si l'`attacker` résolu ci-dessus est le nom d'une
     * invocation connue de ce combat (voir FightParseState.summonOwners), l'action est réattribuée à
     * son invocateur avec le nom de l'invocation comme libellé de "sort" — ex. le Sadida "Fayto"
     * apparaît crédité d'un sort nommé "Dark Lapino" plutôt que "Dark Lapino" d'un sort nommé "Murmures
     * d'affaiblissement" (voir CLAUDE.md, section invocations : une invocation est traitée comme un
     * sort de son invocateur, jamais comme une entité séparée du combat).
     */
    resolveEffectTail(target, tail, state, options) {
      let element = "Inconnu";
      let effectTag = null;
      for (const tagMatch of tail.matchAll(TAG_RE)) {
        const tag = tagMatch[1];
        if (DAMAGE_ELEMENTS.has(tag)) {
          if (element === "Inconnu") element = tag;
        } else if (tag !== IGNORED_TAG) {
          effectTag = tag;
        }
      }
      let attacker = state.lastCast?.caster ?? (options.selfFallback ? target : "Inconnu");
      let spell = state.lastCast?.spell ?? "Autre";
      if (effectTag) {
        const owner = state.effectOwners.get(effectTag.toLowerCase());
        if (owner) {
          attacker = owner.carrier === target ? owner.applier : owner.carrier;
        } else if (options.riposteFallback) {
          const caster = state.spellCasters.get(effectTag.toLowerCase());
          if (caster) {
            attacker = caster;
          } else if (state.lastDamage && state.lastDamage.attacker === target) {
            attacker = state.lastDamage.target;
          }
        } else {
          attacker = target;
        }
        spell = effectTag;
      }
      const rootOwner = this.resolveSummonRootOwner(state, attacker);
      if (rootOwner) {
        spell = attacker;
        attacker = rootOwner;
      }
      return { attacker, spell, element };
    }
    /**
     * Remonte la chaîne `summonOwners` jusqu'à son sommet (un nom qui n'est lui-même l'invocation de
     * personne) plutôt qu'une seule résolution — bug réel corrigé le 2026-08-24, cas "boss qui
     * réinstancie une invocation déjà connue via sa propre annonce Invoque" (ex. fichier Fayto,
     * fightId 1680001273 : "Glouto" — vrai ennemi, jamais lui-même une invocation — meurt puis
     * réapparaît via "Résidu: Invoque un(e) Glouto", "Résidu" étant DÉJÀ une invocation connue de
     * "Druidre" ; sans ce parcours, une seule résolution attribuait l'action à "Résidu" — qui n'a
     * lui-même jamais rejoint `fight.enemies`/`fight.allies` en tant qu'invocation — laissant
     * `attacker` pointer vers un nom fantôme, absent du récap mais réapparaissant comme ligne à part
     * via le repli de `buildEntityDamageRows`, CLAUDE.md). Garde anti-cycle (`visited`) au cas où deux
     * invocations finiraient par se désigner mutuellement comme invocateur (non observé en pratique,
     * mais `summonOwners` est alimenté par du texte de log, pas une structure garantie acyclique).
     * Renvoie `null` si `name` n'est l'invocation de personne (cas de très loin le plus fréquent).
     */
    resolveSummonRootOwner(state, name) {
      let current = name;
      let owner = state.summonOwners.get(current);
      if (!owner) return null;
      const visited = /* @__PURE__ */ new Set([current]);
      while (!visited.has(owner)) {
        visited.add(owner);
        current = owner;
        const next = state.summonOwners.get(current);
        if (!next) break;
        owner = next;
      }
      return owner;
    }
    /** Ignore les doublons stricts (même type d'événement, mêmes champs hors horodatage) survenant dans un intervalle très court — signature d'une observation multi-compte d'un même combat/échange, où chaque compte connecté loggue sa propre copie du flux serveur. */
    isDuplicate(entry) {
      if (DEDUPE_EXEMPT_KINDS.has(entry.kind)) return false;
      const { time, ...rest } = entry;
      void time;
      if (entry.kind === "trade-completed") {
        rest.sides = [...rest.sides].sort(
          (a, b) => a.playerName.localeCompare(b.playerName)
        );
      }
      if (entry.kind === "combat-end") {
        delete rest.result;
      }
      const signature = `${entry.kind}|${JSON.stringify(rest)}`;
      const nowMs = this.timeToMs(entry.time);
      const previous = this.recentSignatures.get(signature);
      this.recentSignatures.set(signature, nowMs);
      if (this.recentSignatures.size > 500) this.pruneSignatures(nowMs);
      return previous !== void 0 && nowMs - previous >= 0 && nowMs - previous <= DEDUPE_WINDOW_MS;
    }
    pruneSignatures(nowMs) {
      for (const [key, seenAt] of this.recentSignatures) {
        if (nowMs - seenAt > DEDUPE_WINDOW_MS) this.recentSignatures.delete(key);
      }
    }
    timeToMs(time) {
      const match = /^(\d{2}):(\d{2}):(\d{2}),(\d{3})$/.exec(time);
      if (!match) return 0;
      const [, h, m, s, ms] = match;
      return ((+h * 60 + +m) * 60 + +s) * 1e3 + +ms;
    }
  };

  // src/entry.ts
  var parser = new LogParser();
  function serialize(entry) {
    return entry === null || entry === void 0 ? "" : JSON.stringify(entry);
  }
  function parseLine(rawLine) {
    return serialize(parser.parseLine(rawLine));
  }
  function flush() {
    return serialize(parser.flush());
  }
  function resetParser() {
    parser.reset();
  }
  function parseBatch(linesJoined) {
    const out = [];
    for (const line of linesJoined.split("\n")) {
      const entry = parser.parseLine(line);
      if (entry !== null) out.push(entry);
    }
    return JSON.stringify(out);
  }
  function parseBatchCountOnly(linesJoined) {
    let count = 0;
    for (const line of linesJoined.split("\n")) {
      if (parser.parseLine(line) !== null) count++;
    }
    return count;
  }
  globalThis.wakfuEngine = { parseLine, flush, resetParser, parseBatch, parseBatchCountOnly };
})();
