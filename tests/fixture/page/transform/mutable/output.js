import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONProps as _withSuperJSONProps } from "superjson-next/tools";
let foo = 1;
foo = 2;
export { foo as getServerSideProps };
foo = _withSuperJSONProps(()=>{}, []);
export default _withSuperJSONPage(()=>{});
