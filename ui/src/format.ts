export function formatWeight(kg: number): string {
  return kg < 1 ? `${Math.round(kg * 1000)} г` : `${kg.toFixed(2)} кг`;
}

export function formatClock(hour: number): string {
  const h = Math.floor(hour);
  const m = Math.floor((hour - h) * 60);
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function plural(n: number, one: string, few: string, many: string): string {
  const m10 = n % 10;
  const m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return one;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few;
  return many;
}

export function formatMoney(rub: number): string {
  return `${Math.round(rub).toLocaleString("ru-RU")} ₽`;
}

/** "День 3, 06:40" from game minutes since day 1, 00:00. */
export function formatGameTime(min: number): string {
  return `День ${Math.floor(min / 1440) + 1}, ${formatClock((min / 60) % 24)}`;
}
