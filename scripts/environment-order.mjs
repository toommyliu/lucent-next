import { readFileSync, writeFileSync } from "node:fs";
import assert from "node:assert/strict";
const base = new Intl.Collator(undefined, { sensitivity: "base" });
const full = new Intl.Collator();
const chars = Array.from({ length: 128 }, (_, i) => String.fromCharCode(i));
const sorted = [...chars].sort(base.compare);
let rank = 0;
let previous = "";
const primary = Array(128).fill(0);
for (const c of sorted) {
  if (base.compare(previous, c)) rank++;
  primary[c.charCodeAt(0)] = rank;
  previous = c;
}
const tertiary = chars.map(c => /[A-Z]/.test(c) ? 2 : /[a-z]/.test(c) ? 1 : 0);
const compare = (a, b) => {
  const weights = s => [...s].filter(c => primary[c.charCodeAt(0)] !== 0);
  const left = weights(a), right = weights(b);
  for (let i = 0; i < Math.min(left.length, right.length); i++) {
    const order = primary[left[i].charCodeAt(0)] - primary[right[i].charCodeAt(0)];
    if (order) return Math.sign(order);
  }
  if (left.length !== right.length) return Math.sign(left.length - right.length);
  for (let i = 0; i < left.length; i++) {
    const order = tertiary[left[i].charCodeAt(0)] - tertiary[right[i].charCodeAt(0)];
    if (order) return Math.sign(order);
  }
  return 0;
};
const names = ["Elders Blood", "Blade of Awe", "XP Boost", "Gold Boost", "apple", "Apple", "APPLE", "Apple blade", "apple Z", "Darkon's Receipt", "Darkon Receipt", "a-b", "ab", "a b", "A!", "a?", "!Blade", "[Blade]", "10th Token", "2nd Token", "Zorbak"];
const pairs = [...chars.flatMap(a => chars.map(b => [a,b])), ...names.flatMap(a => names.map(b => [a,b]))];
for (const [a,b] of pairs) assert.equal(compare(a,b), Math.sign(full.compare(a,b)), JSON.stringify([a,b]));
const table = `pub(super) const PRIMARY: [u8; 128] = [${primary.join(",")}];\npub(super) const TERTIARY: [u8; 128] = [${tertiary.join(",")}];\n`;
const fixtures = JSON.stringify(pairs.map(([a,b]) => [a,b,Math.sign(full.compare(a,b))])) + "\n";
const tablePath = new URL("../crates/host/src/sessions/environment_order.rs", import.meta.url);
const fixturePath = new URL("../packages/schemas/test-fixtures/environment/order.json", import.meta.url);
if (process.argv.includes("--check")) {
  const existing = readFileSync(tablePath,"utf8").replace(/\s/g, "").replace(/,]/g,"]");
  assert.equal(existing, table.replace(/\s/g, "").replace(/,]/g,"]"));
  assert.equal(readFileSync(fixturePath,"utf8"), fixtures);
} else { writeFileSync(tablePath,table); writeFileSync(fixturePath,fixtures); }
console.log(`ASCII pairs: ${128 ** 2}; item-name pairs: ${names.length ** 2}; all match Node Intl.Collator()`);
