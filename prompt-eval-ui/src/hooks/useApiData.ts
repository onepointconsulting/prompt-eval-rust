"use client";

import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

type ApiState<T> = {
  data: T | null;
  loading: boolean;
  error: string | null;
};

export function useApiData<T>(
  loader: () => Promise<T>,
  fallback: T,
  label: string
): ApiState<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const fallbackRef = useRef(fallback);

  useEffect(() => {
    let active = true;
    loader()
      .then((res) => {
        if (!active) return;
        setData(res);
      })
      .catch((err: unknown) => {
        if (!active) return;
        const message = err instanceof Error ? err.message : `Failed to load ${label}`;
        setError(message);
        setData(fallbackRef.current);
        toast.error(`${label} could not be loaded.`);
      })
      .finally(() => {
        if (!active) return;
        setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [label, loader]);

  return { data, loading, error };
}
