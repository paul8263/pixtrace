export interface SearchChip {
  text: string;
  phrase: boolean;
  exclude: boolean;
}

export type SearchChipState = "positive" | "negative";
export type AddTermAction = "added" | "replaced" | "exists" | "empty";

export interface AddTermResult {
  query: string;
  action: AddTermAction;
}

export function chipKey(chip: SearchChip): string {
  return `${chip.exclude ? 1 : 0}:${chip.phrase ? 1 : 0}:${chip.text.toLowerCase()}`;
}

export function serializeChip(chip: SearchChip): string {
  const text = chip.text.replace(/"/g, "");
  const body = chip.phrase ? `"${text}"` : text;
  return `${chip.exclude ? "-" : ""}${body}`;
}

export function serializeQuery(tokens: SearchChip[], draft = ""): string {
  const parts = tokens.map(serializeChip);
  const current = draft.trim();
  if (current) parts.push(current);
  return parts.join(" ");
}

export function parseTerms(input: string): SearchChip[] {
  const chars = [...input];
  const result: SearchChip[] = [];
  let index = 0;
  while (index < chars.length) {
    while (index < chars.length && /\s/.test(chars[index])) index += 1;
    if (index >= chars.length) break;

    let exclude = false;
    if (chars[index] === "-") {
      exclude = true;
      index += 1;
    }
    if (index >= chars.length) break;

    let text = "";
    let phrase = false;
    if (chars[index] === '"') {
      phrase = true;
      index += 1;
      while (index < chars.length && chars[index] !== '"') {
        text += chars[index];
        index += 1;
      }
      if (index < chars.length) index += 1;
    } else {
      while (index < chars.length && !/\s/.test(chars[index])) {
        text += chars[index];
        index += 1;
      }
    }
    const trimmed = text.trim();
    if (trimmed) result.push({ text: trimmed, phrase, exclude });
  }
  return result;
}

export function parseSingleTerm(input: string): SearchChip | null {
  let text = input.trim();
  if (!text) return null;
  let exclude = false;
  if (text.startsWith("-")) {
    exclude = true;
    text = text.slice(1).trim();
  }
  let phrase = false;
  if (text.startsWith('"') && text.endsWith('"') && text.length >= 2) {
    phrase = true;
    text = text.slice(1, -1).trim();
  } else if (/\s/.test(text)) {
    phrase = true;
  }
  text = text.replace(/"/g, "").trim();
  if (!text) return null;
  return { text, phrase, exclude };
}

export function queryTermState(
  query: string,
  text: string,
): SearchChipState | null {
  const needle = text.trim().toLowerCase();
  if (!needle) return null;
  for (const token of parseTerms(query)) {
    if (token.text.toLowerCase() === needle) {
      return token.exclude ? "negative" : "positive";
    }
  }
  return null;
}

export function addQueryTerm(
  query: string,
  rawText: string,
  exclude: boolean,
): AddTermResult {
  const text = rawText.trim().replace(/"/g, "");
  if (!text) return { query, action: "empty" };

  const tokens = parseTerms(query);
  const needle = text.toLowerCase();
  const existing = tokens.filter(
    (token) => token.text.toLowerCase() === needle,
  );
  if (existing.some((token) => token.exclude === exclude)) {
    return { query, action: "exists" };
  }

  const next = tokens.filter((token) => token.text.toLowerCase() !== needle);
  next.push({ text, phrase: /\s/.test(text), exclude });
  return {
    query: next.map(serializeChip).join(" "),
    action: existing.length > 0 ? "replaced" : "added",
  };
}
