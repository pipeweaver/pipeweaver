const MINIMUM_DB = -80.0;
const MAXIMUM_DB = 20.0;

export function dbToLinear(db) {
  return Math.exp((db / 20.0) * Math.LN10);
}

export function linearToDb(amp) {
  if (amp <= 0) return MINIMUM_DB;
  return Math.min(MAXIMUM_DB, Math.max(MINIMUM_DB, 20.0 * Math.log10(amp)));
}
