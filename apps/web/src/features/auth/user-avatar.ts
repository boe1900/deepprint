export function userAvatarInitial(name: string) {
  return name.trim().charAt(0).toUpperCase() || "U"
}

export function userAvatarTone(value: string) {
  const tones = [
    "bg-blue-100 text-blue-700",
    "bg-emerald-100 text-emerald-700",
    "bg-violet-100 text-violet-700",
    "bg-amber-100 text-amber-700",
    "bg-rose-100 text-rose-700",
  ]
  let hash = 0
  for (let index = 0; index < value.length; index += 1) {
    hash = value.charCodeAt(index) + ((hash << 5) - hash)
  }
  return tones[Math.abs(hash) % tones.length]
}
