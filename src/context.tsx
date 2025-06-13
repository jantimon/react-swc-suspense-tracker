import { Suspense, use, useMemo } from "react";
import { SuspenseContext, type BoundaryInfo } from "./internal";

interface BoundaryTrackerProps extends React.ComponentProps<typeof Suspense> {
  boundaryId: string;
  boundary: React.ComponentType<any>;
}

/**
 * Internal component that replaces boundary components via SWC plugin transformation.
 * Provides tracking context while maintaining all original boundary functionality.
 */
export const BoundaryTrackerSWC = ({
  boundaryId,
  boundary: Boundary,
  ...boundaryProps
}: BoundaryTrackerProps) => {
  const parentContext = use(SuspenseContext);
  const boundaries = useMemo<BoundaryInfo[]>(
    () => [[boundaryId, Boundary], ...parentContext],
    [parentContext, boundaryId, Boundary],
  );
  return (
    <SuspenseContext.Provider value={boundaries}>
      <Boundary {...boundaryProps} />
    </SuspenseContext.Provider>
  );
};
