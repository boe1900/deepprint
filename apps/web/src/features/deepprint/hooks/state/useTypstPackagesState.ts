import { useState } from "react";
import type { TypstPackageInfo } from "../../types";

export function useTypstPackagesState() {
  const [packages, setPackages] = useState<TypstPackageInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [deletingKey, setDeletingKey] = useState<string | null>(null);
  const [clearingPreviewCache, setClearingPreviewCache] = useState(false);

  return {
    packages,
    setPackages,
    loading,
    setLoading,
    error,
    setError,
    installing,
    setInstalling,
    deletingKey,
    setDeletingKey,
    clearingPreviewCache,
    setClearingPreviewCache,
  };
}
