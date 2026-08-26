// ─── why ────────────────────────────────────────────────────────
// A zoneless, signals-first template binds `ionInput` / `ionBlur` to a handler
// and gets a DOM `Event`, so every one of them has to reach through
// `event.target` to the value — one cast, in one place.
//
// It lives in `util` and not on a component because it is pure and knows
// nothing about the app: `type:smart-ui → type:util` is allowed, and the
// `@Injectable`-in-util ban does not apply to a plain function.
//
// `target` is the `<ion-input>` HOST, not a native `<input>`. It carries the
// same `value` property, but Ionic types it as `string | number | null`, and a
// cleared input really does read `null`. The coalesce is that case, not
// defensive padding.
// ────────────────────────────────────────────────────────────────

export function inputValue(event: Event): string {
  const { value } = event.target as { value?: string | number | null };
  return value == null ? '' : String(value);
}
