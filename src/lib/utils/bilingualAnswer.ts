/**
 * Splits an "EN: … / ES: …" answer (the copilot's bilingual format) into its
 * two parts. Works on partial, still-streaming text: until the "ES:" label
 * arrives, everything is English. Answers without labels come back as `en`.
 */
export const splitBilingualAnswer = (
  answer: string,
): { en: string; es: string } => {
  const esMatch = /(^|\n)\s*ES:\s*/i.exec(answer);
  const enPart = esMatch ? answer.slice(0, esMatch.index) : answer;
  const esPart = esMatch ? answer.slice(esMatch.index + esMatch[0].length) : "";
  return {
    en: enPart.replace(/^\s*EN:\s*/i, "").trim(),
    es: esPart.trim(),
  };
};
