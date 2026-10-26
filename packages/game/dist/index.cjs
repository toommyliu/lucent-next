Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
// src/model.ts
const normalizeGameText = (value) => value.trim().toLowerCase();
var LiveModel = class {
	#data;
	constructor(data) {
		this.#data = data;
	}
	get modelData() {
		return this.#data;
	}
	/** @internal */
	update(patch) {
		Object.assign(this.#data, patch);
	}
	/** @internal */
	replaceFrom(model) {
		if (this.#data === model.#data) return;
		for (const key of Reflect.ownKeys(this.#data)) Reflect.deleteProperty(this.#data, key);
		Object.assign(this.#data, model.#data);
	}
	/** @internal */
	snapshot() {
		return { ...this.#data };
	}
};
// src/aura.ts
var LiveAura = class extends LiveModel {
	get category() {
		return this.modelData.category;
	}
	/** Full duration in seconds, or `0` when the aura has no timer. */
	get duration() {
		return this.modelData.duration;
	}
	/** Expiry time in epoch milliseconds, or `undefined` when the aura has no timer. The aura stays until the server removes it, even after this time. */
	get expiresAt() {
		return this.modelData.expiresAt;
	}
	get icon() {
		return this.modelData.icon;
	}
	get kind() {
		return this.modelData.kind;
	}
	get name() {
		return this.modelData.name;
	}
	get persistent() {
		return this.modelData.persistent;
	}
	get stack() {
		return this.modelData.stack;
	}
	get value() {
		return this.modelData.value;
	}
	toJSON() {
		return this.snapshot();
	}
};
/** @internal */
function reduceActiveAura(before, edit, nowMs) {
	switch (edit.type) {
		case "seed": {
			const { entry } = edit;
			return {
				...before?.category === void 0 ? {} : { category: before.category },
				...before?.value === void 0 ? {} : { value: before.value },
				name: entry.name,
				kind: "active",
				stack: Math.max(1, entry.stack),
				icon: entry.icon,
				persistent: entry.persistent,
				...entry.timer.type === "untimed" ? { duration: 0 } : {
					duration: Math.max(entry.timer.fullSeconds, entry.timer.remainingSeconds),
					expiresAt: nowMs + entry.timer.remainingSeconds * 1e3
				}
			};
		}
		case "apply": {
			const { entry } = edit;
			const next = {
				...before,
				name: entry.name,
				kind: "active",
				stack: entry.stack,
				persistent: entry.persistent,
				duration: 0
			};
			if (entry.icon !== void 0) next.icon = entry.icon;
			if (entry.category !== void 0) next.category = entry.category;
			if (entry.value !== void 0) next.value = entry.value;
			if (entry.timing.type === "seconds") {
				next.duration = entry.restartDuration ? entry.timing.duration : Math.max(before?.duration ?? 0, entry.timing.duration);
				next.expiresAt = nowMs + entry.timing.duration * 1e3;
			} else delete next.expiresAt;
			return next;
		}
		case "remove": return;
		case "decay": {
			if (before === void 0) return void 0;
			const next = {
				...before,
				stack: edit.stack
			};
			if (edit.refreshSeconds !== void 0 && before.expiresAt !== void 0) {
				next.duration = Math.max(before.duration, edit.refreshSeconds);
				next.expiresAt = nowMs + edit.refreshSeconds * 1e3;
			}
			return next;
		}
		case "set-stack": return before === void 0 ? void 0 : {
			...before,
			stack: Math.max(1, edit.stack)
		};
	}
}
// src/enhancement.ts
const BASIC_HIGH_TIER_MINIMUM_PLAYER_LEVEL = 50;
const NO_PATTERN_ID = 0;
const NO_PROC_ID = 0;
const PATTERN_IDS = {
	absolution: 11,
	anima: 28,
	avarice: 12,
	depths: 23,
	examen: 26,
	fighter: 2,
	forge: 10,
	healer: 7,
	hearty: 32,
	hybrid: 5,
	lament: 30,
	lucky: 9,
	penitence: 29,
	pneuma: 27,
	spellbreaker: 8,
	thief: 3,
	vainglory: 24,
	vim: 25,
	wizard: 6
};
const WEAPON_PROC_IDS = {
	acheron: 11,
	arcanaConcerto: 10,
	aweBlast: 3,
	dauntless: 14,
	elysium: 12,
	healthVamp: 4,
	lacerate: 7,
	manaVamp: 5,
	powerwordDie: 6,
	praxis: 13,
	ravenous: 15,
	smite: 8,
	spiralCarve: 2,
	valiance: 9
};
const FORGE_SHOP_IDS = {
	cape: 2143,
	helm: 2164,
	weapon: 2142
};
const BASIC_ENHANCEMENTS = [
	{
		aweShopId: 635,
		name: "fighter",
		patternId: PATTERN_IDS.fighter,
		shopIds: {
			high: 768,
			low: 141
		}
	},
	{
		aweShopId: 637,
		name: "thief",
		patternId: PATTERN_IDS.thief,
		shopIds: {
			high: 767,
			low: 142
		}
	},
	{
		aweShopId: 633,
		name: "hybrid",
		patternId: PATTERN_IDS.hybrid,
		shopIds: {
			high: 766,
			low: 143
		}
	},
	{
		aweShopId: 636,
		name: "wizard",
		patternId: PATTERN_IDS.wizard,
		shopIds: {
			high: 765,
			low: 144
		}
	},
	{
		aweShopId: 638,
		name: "healer",
		patternId: PATTERN_IDS.healer,
		shopIds: {
			high: 762,
			low: 145
		}
	},
	{
		name: "spellbreaker",
		patternId: PATTERN_IDS.spellbreaker,
		shopIds: {
			high: 764,
			low: 146
		}
	},
	{
		aweShopId: 639,
		name: "lucky",
		patternId: PATTERN_IDS.lucky,
		shopIds: {
			high: 763,
			low: 147
		}
	}
];
const WEAPON_SPECIALS = [
	{
		aliases: ["scarve", "spiral"],
		family: "awe",
		name: "spiral carve",
		procId: WEAPON_PROC_IDS.spiralCarve
	},
	{
		aliases: [
			"ablast",
			"aweblast",
			"blast"
		],
		family: "awe",
		name: "awe blast",
		procId: WEAPON_PROC_IDS.aweBlast
	},
	{
		aliases: [
			"healthvamp",
			"hvamp",
			"hp vamp"
		],
		family: "awe",
		name: "health vamp",
		procId: WEAPON_PROC_IDS.healthVamp
	},
	{
		aliases: [
			"manavamp",
			"mvamp",
			"mp vamp"
		],
		family: "awe",
		name: "mana vamp",
		procId: WEAPON_PROC_IDS.manaVamp
	},
	{
		aliases: [
			"powerword",
			"pwd",
			"pw die"
		],
		displayName: "Powerword DIE",
		family: "awe",
		name: "powerword die",
		procId: WEAPON_PROC_IDS.powerwordDie
	},
	{
		aliases: ["lac"],
		family: "forge",
		name: "lacerate",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.lacerate
	},
	{
		family: "forge",
		name: "smite",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.smite
	},
	{
		aliases: ["val", "vali"],
		family: "forge",
		name: "valiance",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.valiance
	},
	{
		aliases: [
			"arcanas",
			"arcana concerto",
			"concerto"
		],
		family: "forge",
		name: "arcana's concerto",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.arcanaConcerto
	},
	{
		aliases: ["ach"],
		family: "forge",
		name: "acheron",
		patternId: PATTERN_IDS.depths,
		procId: WEAPON_PROC_IDS.acheron
	},
	{
		aliases: ["ely"],
		family: "forge",
		name: "elysium",
		patternId: PATTERN_IDS.wizard,
		procId: WEAPON_PROC_IDS.elysium
	},
	{
		aliases: ["prax"],
		family: "forge",
		name: "praxis",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.praxis
	},
	{
		aliases: ["dtl"],
		family: "forge",
		name: "dauntless",
		patternId: PATTERN_IDS.fighter,
		procId: WEAPON_PROC_IDS.dauntless
	},
	{
		aliases: ["rav"],
		family: "forge",
		name: "ravenous",
		patternId: PATTERN_IDS.forge,
		procId: WEAPON_PROC_IDS.ravenous
	}
];
const FORGE_WEAPON_PATTERNS = [{
	name: "forge",
	patternId: PATTERN_IDS.forge
}, {
	name: "depths",
	patternId: PATTERN_IDS.depths
}];
const CAPE_SPECIALS = [
	{
		name: "forge",
		patternId: PATTERN_IDS.forge
	},
	{
		aliases: ["abso"],
		name: "absolution",
		patternId: PATTERN_IDS.absolution
	},
	{
		aliases: ["ava"],
		name: "avarice",
		patternId: PATTERN_IDS.avarice
	},
	{
		name: "vainglory",
		patternId: PATTERN_IDS.vainglory
	},
	{
		aliases: ["peni"],
		name: "penitence",
		patternId: PATTERN_IDS.penitence
	},
	{
		aliases: ["lam"],
		name: "lament",
		patternId: PATTERN_IDS.lament
	}
];
const HELM_ENHANCEMENTS = [
	{
		name: "vim",
		patternId: PATTERN_IDS.vim,
		special: { name: "ether" }
	},
	{
		name: "examen",
		patternId: PATTERN_IDS.examen,
		special: { name: "ether" }
	},
	{
		name: "pneuma",
		patternId: PATTERN_IDS.pneuma,
		special: { name: "clairvoyance" }
	},
	{
		name: "anima",
		patternId: PATTERN_IDS.anima,
		special: { name: "clairvoyance" }
	},
	{
		inventoryName: "grimskull",
		name: "hearty",
		patternId: PATTERN_IDS.hearty,
		special: { name: "hearty" }
	}
];
const EXTRA_PATTERN_DISPLAY_NAMES = /* @__PURE__ */ new Map([[1, "Adventurer"], [4, "Armsman"]]);
const titleCase = (value) => value.split(" ").map((word) => word === "" ? word : `${word[0].toUpperCase()}${word.slice(1)}`).join(" ");
const namedValueDisplayName = (value) => value.displayName ?? titleCase(value.name);
const enhancementPatternDisplayNames = (patternId) => {
	const basic = BASIC_ENHANCEMENTS.find((entry) => entry.patternId === patternId);
	if (basic !== void 0) return [namedValueDisplayName(basic)];
	const weapon = FORGE_WEAPON_PATTERNS.find((entry) => entry.patternId === patternId);
	if (weapon !== void 0) return [namedValueDisplayName(weapon)];
	const cape = CAPE_SPECIALS.find((entry) => entry.patternId === patternId);
	if (cape !== void 0) {
		const name = namedValueDisplayName(cape);
		return name === "Forge" ? [name] : ["Forge", name];
	}
	const helm = HELM_ENHANCEMENTS.find((entry) => entry.patternId === patternId);
	if (helm !== void 0) return [titleCase("inventoryName" in helm ? helm.inventoryName : helm.name), namedValueDisplayName(helm.special)];
	const extra = EXTRA_PATTERN_DISPLAY_NAMES.get(patternId);
	return extra === void 0 ? void 0 : [extra];
};
const getItemEnhancementNames = (enhancement) => {
	const patternId = enhancement.patternId;
	const [name, patternSpecial] = patternId !== void 0 && patternId > 0 ? enhancementPatternDisplayNames(patternId) ?? [`Pattern ${patternId}`] : [];
	const procId = enhancement.procId;
	const proc = WEAPON_SPECIALS.find((entry) => entry.procId === procId);
	return {
		name,
		special: procId !== void 0 && procId > 0 ? proc === void 0 ? `Proc ${procId}` : namedValueDisplayName(proc) : patternSpecial
	};
};
const formatItemEnhancement = (enhancement) => {
	if (enhancement === void 0) return;
	const { name, special } = getItemEnhancementNames(enhancement);
	const parts = [];
	if (name !== void 0) parts.push(name);
	if (special !== void 0) parts.push(special);
	const level = enhancement.level;
	if (level !== void 0 && level > 0) parts.push(`Level ${level}`);
	return parts.length === 0 ? void 0 : parts.join(", ");
};
const normalize = (value) => value.trim().toLowerCase().replaceAll(/[‘’]/gu, "'").replaceAll(/\s+/gu, " ");
const matchesNamedValue = (entry, value) => {
	const requested = normalize(value);
	return entry.name === requested || entry.aliases?.some((alias) => alias === requested) === true;
};
const resolveNamedValue = (entries, value) => entries.find((entry) => matchesNamedValue(entry, value));
const resolvePatternId = (entries, value) => resolveNamedValue(entries, value)?.patternId;
const inventoryHelmName = (entry) => entry.inventoryName ?? entry.name;
const resolveInventoryHelmEnhancement = (enhancement, special) => HELM_ENHANCEMENTS.find((entry) => normalize(inventoryHelmName(entry)) === normalize(enhancement) && matchesNamedValue(entry.special, special));
const enhancementSlot = (item) => {
	if (item.weapon) return "weapon";
	if (item.cape) return "cape";
	if (item.helm) return "helm";
	if (item.classItem) return "class";
};
const basicStrategy = (enhancement, playerLevel, slot) => ({
	ok: true,
	strategy: {
		patternId: enhancement.patternId,
		procId: NO_PROC_ID,
		shopId: enhancement.shopIds[playerLevel >= BASIC_HIGH_TIER_MINIMUM_PLAYER_LEVEL ? "high" : "low"],
		slot
	}
});
const resolveEnhancementStrategy = (item, enhancement, playerLevel, special) => {
	const slot = enhancementSlot(item);
	if (slot === void 0) return {
		ok: false,
		reason: "The selected item cannot be enhanced"
	};
	const requested = normalize(enhancement);
	const requestedSpecial = normalize(special ?? "");
	const basicEnhancement = resolveNamedValue(BASIC_ENHANCEMENTS, requested);
	const forge = requested === "forge";
	if (slot === "class") {
		if (forge || requestedSpecial !== "" || basicEnhancement === void 0) return {
			ok: false,
			reason: "The requested enhancement cannot be applied to a class"
		};
		return basicStrategy(basicEnhancement, playerLevel, slot);
	}
	if (slot === "weapon") {
		const explicitSpecial = requestedSpecial === "" ? void 0 : resolveNamedValue(WEAPON_SPECIALS, requestedSpecial);
		if (requestedSpecial !== "" && explicitSpecial === void 0) return {
			ok: false,
			reason: "Unknown weapon enhancement special"
		};
		const shorthandSpecial = requestedSpecial === "" ? resolveNamedValue(WEAPON_SPECIALS, requested) : void 0;
		const weaponSpecial = explicitSpecial ?? (shorthandSpecial?.family === "forge" ? shorthandSpecial : void 0);
		const requestedPatternId = basicEnhancement?.patternId ?? resolvePatternId(FORGE_WEAPON_PATTERNS, requested);
		if (weaponSpecial === void 0) {
			if (basicEnhancement !== void 0) return basicStrategy(basicEnhancement, playerLevel, slot);
			if (requestedPatternId !== PATTERN_IDS.forge) return {
				ok: false,
				reason: "Unknown weapon enhancement"
			};
			return {
				ok: true,
				strategy: {
					map: "forge",
					patternId: PATTERN_IDS.forge,
					procId: NO_PROC_ID,
					shopId: FORGE_SHOP_IDS.weapon,
					slot
				}
			};
		}
		if (weaponSpecial.family === "awe") {
			if (basicEnhancement === void 0) return {
				ok: false,
				reason: "Awe specials require a basic enhancement"
			};
			const shopId = basicEnhancement.aweShopId;
			return shopId === void 0 ? {
				ok: false,
				reason: "No Awe enhancement shop matched the request"
			} : {
				ok: true,
				strategy: {
					map: "museum",
					patternId: basicEnhancement.patternId,
					procId: weaponSpecial.procId,
					shopId,
					slot
				}
			};
		}
		if (!(forge && explicitSpecial !== void 0) && !(shorthandSpecial === weaponSpecial) && requestedPatternId !== weaponSpecial.patternId) return {
			ok: false,
			reason: "The enhancement and special do not match their inventory label"
		};
		return {
			ok: true,
			strategy: {
				map: "forge",
				patternId: weaponSpecial.patternId,
				procId: weaponSpecial.procId,
				shopId: FORGE_SHOP_IDS.weapon,
				slot
			}
		};
	}
	if (slot === "helm") {
		if (basicEnhancement !== void 0 && requestedSpecial === "") return basicStrategy(basicEnhancement, playerLevel, slot);
		if (forge && requestedSpecial === "") return {
			ok: true,
			strategy: {
				map: "forge",
				patternId: PATTERN_IDS.forge,
				procId: NO_PROC_ID,
				shopId: FORGE_SHOP_IDS.helm,
				slot
			}
		};
		const helmEnhancement = forge ? resolveNamedValue(HELM_ENHANCEMENTS, requestedSpecial) : requestedSpecial === "" ? resolveNamedValue(HELM_ENHANCEMENTS, requested) : resolveInventoryHelmEnhancement(requested, requestedSpecial);
		if (helmEnhancement === void 0) return {
			ok: false,
			reason: "Unknown Forge helm enhancement"
		};
		return {
			ok: true,
			strategy: {
				map: "forge",
				patternId: helmEnhancement.patternId,
				procId: NO_PROC_ID,
				shopId: FORGE_SHOP_IDS.helm,
				slot
			}
		};
	}
	if (basicEnhancement !== void 0 && requestedSpecial === "") return basicStrategy(basicEnhancement, playerLevel, slot);
	const shorthandCape = requestedSpecial === "" ? resolveNamedValue(CAPE_SPECIALS, requested) : void 0;
	const patternId = forge ? requestedSpecial === "" ? PATTERN_IDS.forge : resolvePatternId(CAPE_SPECIALS, requestedSpecial) : shorthandCape?.patternId;
	if (patternId === void 0) return {
		ok: false,
		reason: "Unknown Forge cape enhancement"
	};
	return {
		ok: true,
		strategy: {
			map: "forge",
			patternId,
			procId: NO_PROC_ID,
			shopId: FORGE_SHOP_IDS.cape,
			slot
		}
	};
};
const normalizedEquipmentSlot = (item) => normalize(item.equipmentSlot);
const matchesSlot = (item, slot) => {
	const actual = normalizedEquipmentSlot(item);
	switch (slot) {
		case "weapon": return actual === "weapon";
		case "cape": return actual === "ba" || actual === "cape";
		case "helm": return actual === "he" || actual === "helm";
		case "class": return actual === "ar" || actual === "class";
	}
};
const matchesEnhancementShopItem = (item, strategy) => {
	if (normalize(item.category) !== "enhancement") return false;
	if (!matchesSlot(item, strategy.slot)) return false;
	const patternId = item.enhancement?.patternId ?? NO_PATTERN_ID;
	const procId = item.enhancement?.procId ?? NO_PROC_ID;
	return patternId === strategy.patternId && procId === strategy.procId;
};
const matchesAppliedEnhancement = (item, strategy) => {
	const patternId = item.enhancement?.patternId ?? NO_PATTERN_ID;
	const procId = item.enhancement?.procId ?? NO_PROC_ID;
	return patternId === strategy.patternId && procId === strategy.procId;
};
// src/entity.ts
const percent = (value, maximum) => maximum <= 0 ? 0 : value / maximum * 100;
const EntityState = {
	Dead: 0,
	Idle: 1,
	InCombat: 2
};
const isEntityState = (value) => value === EntityState.Dead || value === EntityState.Idle || value === EntityState.InCombat;
var LiveEntity = class extends LiveModel {
	#auras = /* @__PURE__ */ new Map();
	#acceptsActiveSync = true;
	get alive() {
		return this.hp > 0 && this.state !== EntityState.Dead;
	}
	get auras() {
		return Array.from(this.#auras.values());
	}
	get cell() {
		return this.modelData.cell;
	}
	get dead() {
		return !this.alive;
	}
	get hp() {
		return this.modelData.hp;
	}
	get hpPercent() {
		return percent(this.hp, this.maxHp);
	}
	get idle() {
		return this.state === EntityState.Idle;
	}
	get inCombat() {
		return this.state === EntityState.InCombat;
	}
	get maxHp() {
		return this.modelData.maxHp;
	}
	get maxMp() {
		return this.modelData.maxMp;
	}
	get mp() {
		return this.modelData.mp;
	}
	get mpPercent() {
		return percent(this.mp, this.maxMp);
	}
	get state() {
		return this.modelData.state;
	}
	getAura(name, options) {
		const normalizedName = normalizeGameText(name);
		for (const aura of this.#auras.values()) if (normalizeGameText(aura.name) === normalizedName && (options?.kind === void 0 || aura.kind === options.kind)) return aura;
		return null;
	}
	hasAura(name, options) {
		return this.getAura(name, options) !== null;
	}
	/** @internal */
	projectAuras(mutation, nowMs) {
		const changes = [];
		if (!this.#acceptsActiveSync && (mutation.type === "apply" || mutation.type === "withdraw" || mutation.type === "set-stack")) return changes;
		switch (mutation.type) {
			case "seed": {
				const entries = new Map(mutation.entries.map((entry) => [entry.name, entry]));
				for (const aura of this.#auras.values()) if (aura.kind === "active" && !entries.has(aura.name)) this.commit("active", aura.name, () => void 0, changes);
				for (const entry of entries.values()) this.commit("active", entry.name, (before) => reduceActiveAura(before, {
					type: "seed",
					entry
				}, nowMs), changes);
				break;
			}
			case "apply":
				for (const entry of mutation.entries) this.commit("active", entry.name, (before) => reduceActiveAura(before, {
					type: "apply",
					entry
				}, nowMs), changes);
				break;
			case "withdraw":
				for (const entry of mutation.entries) this.commit("active", entry.name, (before) => reduceActiveAura(before, entry, nowMs), changes);
				break;
			case "set-stack":
				for (const entry of mutation.entries) this.commit("active", entry.name, (before) => reduceActiveAura(before, {
					type: "set-stack",
					...entry
				}, nowMs), changes);
				break;
			case "passives": {
				const entries = new Map(mutation.entries.map((entry) => [entry.name, entry]));
				if (mutation.mode === "replace") {
					for (const aura of this.#auras.values()) if (aura.kind === "passive" && !entries.has(aura.name)) this.commit("passive", aura.name, () => void 0, changes);
				}
				for (const entry of entries.values()) this.commit("passive", entry.name, () => ({
					...entry,
					kind: "passive",
					stack: 1,
					persistent: false
				}), changes);
				break;
			}
			case "clear": for (const aura of this.#auras.values()) if (aura.kind === "active" && (!mutation.keepPersistent || !aura.persistent)) this.commit("active", aura.name, () => void 0, changes);
		}
		return changes;
	}
	/** @internal */
	writeAuraMonsterState(state) {
		this.#acceptsActiveSync = state !== EntityState.Dead;
		return state === EntityState.InCombat ? [] : this.projectAuras({
			type: "clear",
			keepPersistent: false
		}, 0);
	}
	auraKey(kind, name) {
		return `${kind}:${name}`;
	}
	commit(kind, name, edit, changes) {
		const key = this.auraKey(kind, name);
		const current = this.#auras.get(key);
		if (current === void 0) {
			const after = edit(void 0);
			if (after !== void 0) {
				this.#auras.set(key, new LiveAura({ ...after }));
				changes.push({
					type: "added",
					after: { ...after }
				});
			}
			return;
		}
		const before = current.toJSON();
		const after = edit(before);
		if (after === void 0) {
			this.#auras.delete(key);
			changes.push({
				type: "removed",
				before
			});
		} else if (!Object.entries(before).every(([key, value]) => Reflect.get(after, key) === value) || !Object.entries(after).every(([key, value]) => Reflect.get(before, key) === value)) {
			current.replaceFrom(new LiveAura({ ...after }));
			changes.push({
				type: "updated",
				before,
				after: { ...after }
			});
		}
	}
	isInCell(cell) {
		return normalizeGameText(this.cell) === normalizeGameText(cell);
	}
};
// src/faction.ts
var LiveFaction = class extends LiveModel {
	get id() {
		return this.modelData.id;
	}
	get name() {
		return this.modelData.name;
	}
	get rank() {
		return this.modelData.rank;
	}
	get reputation() {
		return this.modelData.reputation;
	}
	toJSON() {
		return { ...this.modelData };
	}
};
// src/item.ts
const miscCategories = /* @__PURE__ */ new Set([
	"Item",
	"Note",
	"Quest Item",
	"Resource"
]);
const itemRarityNames = {
	0: "Unknown",
	1: "Enhancement +0",
	2: "Enhancement +1",
	3: "Enhancement +2",
	4: "Enhancement +3",
	5: "Enhancement +4",
	6: "Enhancement +5",
	7: "Enhancement +6",
	8: "Enhancement +100",
	9: "Enhancement +200",
	10: "Unknown",
	11: "Common",
	12: "Weird",
	13: "Awesome",
	14: "1% Drop",
	15: "5% Drop",
	16: "Boss Drop",
	17: "Hidden Secret",
	18: "Junk",
	19: "Impossible",
	20: "Artifact",
	21: "Limited Time Drop",
	23: "Crazy",
	24: "Expensive",
	25: "Placeholder",
	30: "Rare",
	35: "Epic",
	36: "Champion",
	40: "Verification Shop",
	50: "Seasonal",
	55: "Seasonal Rare",
	60: "Event",
	65: "Event Rare",
	67: "Infinity",
	68: "New Collection Chest",
	70: "Limited Rare",
	75: "Collector",
	80: "Promo",
	81: "Founder",
	88: "Benevolent",
	90: "Ultra Rare",
	91: "Achievement Tracker",
	92: "Upgrade Pack",
	93: "Limited Quantity",
	94: "Frostval Gifting",
	95: "Super Ultra Rare",
	96: "Kickstarter Backer",
	99: "Custom Item",
	100: "Legendary"
};
const getItemRarityName = (rarity) => itemRarityNames[rarity] ?? "Unknown";
const classRankPointThresholds = [
	900,
	3600,
	1e4,
	22500,
	44100,
	78400,
	129600,
	202500,
	302500
];
/** Returns the class rank for a cumulative class-point total. */
const getClassRankFromPoints = (classPoints) => {
	if (!Number.isFinite(classPoints)) return 1;
	const points = Math.max(0, Math.trunc(classPoints));
	let rank = 1;
	for (const threshold of classRankPointThresholds) {
		if (points < threshold) break;
		rank += 1;
	}
	return rank;
};
const toItemSelector = (query) => {
	if (typeof query === "number") return { itemId: query };
	if (typeof query === "string") return { name: query.trim() };
	return "name" in query ? { name: query.name.trim() } : query;
};
const normalizeItemQuantity = (value) => value === void 0 || !Number.isFinite(value) ? 1 : Math.max(1, Math.trunc(value));
var LiveItem = class extends LiveModel {
	get armor() {
		return this.equipmentSlot === "co";
	}
	get banked() {
		return this.context === "bank";
	}
	get cape() {
		return this.equipmentSlot === "ba";
	}
	get category() {
		return this.modelData.category;
	}
	get charItemId() {
		return this.modelData.charItemId;
	}
	get classItem() {
		return this.category === "Class";
	}
	get classRank() {
		return this.classItem ? getClassRankFromPoints(this.quantity) : null;
	}
	get coins() {
		return this.modelData.coins;
	}
	get context() {
		return this.modelData.context;
	}
	get cost() {
		return this.modelData.cost;
	}
	get description() {
		return this.modelData.description;
	}
	get enhancement() {
		const enhancement = this.modelData.enhancement;
		if (enhancement === void 0) return void 0;
		return {
			...enhancement,
			...getItemEnhancementNames(enhancement)
		};
	}
	get equipped() {
		return this.modelData.equipped;
	}
	get equipmentSlot() {
		return this.modelData.equipmentSlot;
	}
	get file() {
		return this.modelData.file;
	}
	get helm() {
		return this.equipmentSlot === "he";
	}
	get houseItem() {
		return this.modelData.houseItem;
	}
	get itemId() {
		return this.modelData.itemId;
	}
	get link() {
		return this.modelData.link;
	}
	get maxStack() {
		return this.modelData.maxStack;
	}
	get memberOnly() {
		return this.modelData.memberOnly;
	}
	get meta() {
		return this.modelData.meta;
	}
	get name() {
		return this.modelData.name;
	}
	get pet() {
		return this.equipmentSlot === "pe";
	}
	get pool() {
		if (this.classItem) return "class";
		if (this.houseItem || this.category === "Guild") return "house";
		return miscCategories.has(this.category) && !(this.category === "Item" && /^\s*\d+\s*$/.test(this.meta)) ? "misc" : "bag";
	}
	get quantity() {
		return this.modelData.quantity;
	}
	get requirements() {
		return this.modelData.requirements ?? [];
	}
	get shopItemId() {
		return this.modelData.shopItemId;
	}
	get temporaryItem() {
		return this.modelData.temporaryItem;
	}
	get weapon() {
		return this.equipmentSlot === "Weapon";
	}
	get wearable() {
		return this.modelData.wearable ?? false;
	}
	get worn() {
		return this.modelData.worn ?? false;
	}
	matches(selector) {
		if (typeof selector === "number") return this.itemId === selector;
		if (typeof selector === "string") return normalizeGameText(this.name) === normalizeGameText(selector);
		if ("itemId" in selector) return this.itemId === selector.itemId;
		if ("name" in selector) return normalizeGameText(this.name) === normalizeGameText(selector.name);
		if ("shopItemId" in selector) return this.shopItemId === selector.shopItemId;
		return false;
	}
	toJSON() {
		const enhancement = this.enhancement;
		return {
			...this.modelData,
			armor: this.armor,
			banked: this.banked,
			cape: this.cape,
			classItem: this.classItem,
			classRank: this.classRank,
			...enhancement === void 0 ? {} : { enhancement },
			helm: this.helm,
			pet: this.pet,
			pool: this.pool,
			...this.modelData.requirements === void 0 ? {} : { requirements: this.requirements.map((requirement) => ({ ...requirement })) },
			wearable: this.wearable,
			weapon: this.weapon,
			worn: this.worn
		};
	}
};
// src/monster.ts
const monsterMapIdToken = /^id[.:'-]([1-9]\d*)$/iu;
const parseMonsterMapId = (value) => {
	const match = value.trim().match(monsterMapIdToken);
	return match?.[1] === void 0 ? void 0 : Number(match[1]);
};
const toMonsterSelector = (query) => {
	if (typeof query === "number") return { monsterMapId: query };
	if (typeof query === "object") return "monsterMapId" in query ? query : { name: query.name.trim() };
	const monsterMapId = parseMonsterMapId(query);
	return monsterMapId === void 0 ? { name: query.trim() } : { monsterMapId };
};
const orderMonstersByPriority = (monsters, priorities) => {
	const ordered = [];
	const selected = /* @__PURE__ */ new Set();
	for (const priority of priorities) for (const monster of monsters) if (!selected.has(monster.monsterMapId) && monster.matches(priority)) {
		ordered.push(monster);
		selected.add(monster.monsterMapId);
	}
	return ordered;
};
var LiveMonster = class extends LiveEntity {
	#drops = /* @__PURE__ */ new Map();
	get aggressive() {
		return this.modelData.aggressive;
	}
	get drops() {
		return Array.from(this.#drops.values());
	}
	get level() {
		return this.modelData.level;
	}
	get monsterId() {
		return this.modelData.monsterId;
	}
	get monsterMapId() {
		return this.modelData.monsterMapId;
	}
	get name() {
		return this.modelData.name;
	}
	get race() {
		return this.modelData.race;
	}
	matches(selector) {
		if (typeof selector === "number") return this.monsterMapId === selector;
		if (typeof selector === "string") {
			const monsterMapId = parseMonsterMapId(selector);
			if (monsterMapId !== void 0) return this.monsterMapId === monsterMapId;
		} else if ("monsterMapId" in selector) return this.monsterMapId === selector.monsterMapId;
		const name = normalizeGameText(typeof selector === "string" ? selector : selector.name);
		return name === "*" || name !== "" && normalizeGameText(this.name) === name;
	}
	/**
	* Replaces the server-provided drop collection without replacing the monster.
	* @internal
	*/
	replaceDrops(drops) {
		this.#drops.clear();
		for (const drop of drops) this.#drops.set(drop.item.itemId, {
			...drop,
			item: { ...drop.item },
			questObjectives: [...drop.questObjectives],
			requiredQuestIds: [...drop.requiredQuestIds],
			requiredQuests: [...drop.requiredQuests]
		});
	}
	toJSON() {
		return {
			...this.modelData,
			alive: this.alive,
			auras: this.auras.map((aura) => aura.toJSON()),
			dead: this.dead,
			drops: this.drops.map((drop) => ({
				...drop,
				item: { ...drop.item },
				questObjectives: [...drop.questObjectives],
				requiredQuestIds: [...drop.requiredQuestIds],
				requiredQuests: [...drop.requiredQuests]
			})),
			hpPercent: this.hpPercent,
			idle: this.idle,
			inCombat: this.inCombat,
			mpPercent: this.mpPercent
		};
	}
};
// src/outfit.ts
var LiveOutfit = class extends LiveModel {
	get colors() {
		return this.modelData.colors;
	}
	get equipment() {
		return this.modelData.equipment;
	}
	get name() {
		return this.modelData.name;
	}
	toJSON() {
		return {
			...this.modelData,
			colors: { ...this.colors },
			equipment: { ...this.equipment }
		};
	}
};
// src/player.ts
const toPlayerSelector = (query) => ({ username: typeof query === "string" ? query.trim() : query.username.trim() });
var LivePlayer = class extends LiveEntity {
	get afk() {
		return this.modelData.afk;
	}
	get entityId() {
		return this.modelData.entityId;
	}
	get entityType() {
		return this.modelData.entityType;
	}
	get level() {
		return this.modelData.level;
	}
	get pad() {
		return this.modelData.pad;
	}
	get position() {
		return this.modelData.position;
	}
	get username() {
		return this.modelData.username;
	}
	toJSON() {
		return {
			...this.modelData,
			alive: this.alive,
			auras: this.auras.map((aura) => aura.toJSON()),
			dead: this.dead,
			hpPercent: this.hpPercent,
			idle: this.idle,
			inCombat: this.inCombat,
			mpPercent: this.mpPercent,
			position: { ...this.position }
		};
	}
};
// src/quest.ts
var LiveQuest = class extends LiveModel {
	/**
	* How often the quest can be completed.
	*/
	get cadence() {
		return this.modelData.cadence;
	}
	get id() {
		return this.modelData.id;
	}
	get name() {
		return this.modelData.name;
	}
	get once() {
		return this.modelData.once;
	}
	get requirements() {
		return this.modelData.requirements;
	}
	get rewards() {
		return this.modelData.rewards;
	}
	toJSON() {
		return {
			...this.modelData,
			requirements: this.requirements.map((item) => ({ ...item })),
			rewards: this.rewards.map((item) => ({ ...item }))
		};
	}
};
// src/server.ts
var LiveServer = class extends LiveModel {
	get chat() {
		return this.modelData.chat;
	}
	get count() {
		return this.modelData.count;
	}
	get full() {
		return this.max > 0 && this.count >= this.max;
	}
	get language() {
		return this.modelData.language;
	}
	get max() {
		return this.modelData.max;
	}
	get memberOnly() {
		return this.modelData.memberOnly;
	}
	get name() {
		return this.modelData.name;
	}
	get online() {
		return this.modelData.online;
	}
	toJSON() {
		return {
			...this.modelData,
			full: this.full
		};
	}
};
// src/shop.ts
var LiveShop = class extends LiveModel {
	get house() {
		return this.modelData.house;
	}
	get id() {
		return this.modelData.id;
	}
	get items() {
		return this.modelData.items;
	}
	get limited() {
		return this.modelData.limited;
	}
	get merge() {
		return this.modelData.merge;
	}
	get name() {
		return this.modelData.name;
	}
	toJSON() {
		return {
			...this.modelData,
			items: this.items.map((item) => item.toJSON())
		};
	}
};
exports.EntityState = EntityState;
exports.LiveAura = LiveAura;
exports.LiveEntity = LiveEntity;
exports.LiveFaction = LiveFaction;
exports.LiveItem = LiveItem;
exports.LiveMonster = LiveMonster;
exports.LiveOutfit = LiveOutfit;
exports.LivePlayer = LivePlayer;
exports.LiveQuest = LiveQuest;
exports.LiveServer = LiveServer;
exports.LiveShop = LiveShop;
exports.formatItemEnhancement = formatItemEnhancement;
exports.getClassRankFromPoints = getClassRankFromPoints;
exports.getItemEnhancementNames = getItemEnhancementNames;
exports.getItemRarityName = getItemRarityName;
exports.isEntityState = isEntityState;
exports.matchesAppliedEnhancement = matchesAppliedEnhancement;
exports.matchesEnhancementShopItem = matchesEnhancementShopItem;
exports.normalizeItemQuantity = normalizeItemQuantity;
exports.orderMonstersByPriority = orderMonstersByPriority;
exports.parseMonsterMapId = parseMonsterMapId;
exports.reduceActiveAura = reduceActiveAura;
exports.resolveEnhancementStrategy = resolveEnhancementStrategy;
exports.toItemSelector = toItemSelector;
exports.toMonsterSelector = toMonsterSelector;
exports.toPlayerSelector = toPlayerSelector;
