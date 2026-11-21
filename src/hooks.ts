import { useCallback, useEffect, useRef, useState } from "react";
import { asIpcError, type IpcError } from "./ipc";

export function useResource<T>(loader: () => Promise<T>) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<IpcError | null>(null);
  const generation = useRef(0);
  const reload = useCallback(async (): Promise<T | null> => {
    const request = ++generation.current;
    setLoading(true);
    setError(null);
    try {
      const value = await loader();
      if (request === generation.current) setData(value);
      return value;
    } catch (cause) {
      if (request === generation.current) setError(asIpcError(cause));
      return null;
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [loader]);

  useEffect(() => {
    void reload();
    return () => { generation.current += 1; };
  }, [reload]);

  const replace = useCallback((value: T) => {
    generation.current += 1;
    setData(value);
    setError(null);
    setLoading(false);
  }, []);

  return { data, loading, error, reload, replace };
}
