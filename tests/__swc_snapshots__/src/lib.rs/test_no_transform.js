import { useEffect, Suspense } from "react";
function App() {
    return <Suspense fallback={<Loading/>}>
      <MyComponent/>
    </Suspense>;
}
