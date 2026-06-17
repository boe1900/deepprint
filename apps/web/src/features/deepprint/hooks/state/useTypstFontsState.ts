import { useState } from "react";
import type { TypstFontInfo } from "../../types";

export function useTypstFontsState() {
  const [fonts, setFonts] = useState<TypstFontInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [deletingName, setDeletingName] = useState<string | null>(null);

  return {
    fonts,
    setFonts,
    loading,
    setLoading,
    error,
    setError,
    installing,
    setInstalling,
    deletingName,
    setDeletingName,
  };
}
