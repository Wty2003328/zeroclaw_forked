import { useState, useEffect, useRef, useCallback } from 'react';

export function useContainerWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  const measure = useCallback(() => {
    if (ref.current) {
      setWidth(ref.current.offsetWidth);
    }
  }, []);

  useEffect(() => {
    measure();

    const observer = new ResizeObserver(() => measure());
    if (ref.current) {
      observer.observe(ref.current);
    }

    return () => observer.disconnect();
  }, [measure]);

  return { ref, width };
}
