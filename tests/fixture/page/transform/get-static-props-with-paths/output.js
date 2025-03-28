import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONProps as _withSuperJSONProps } from "superjson-next/tools";
export const getStaticProps = _withSuperJSONProps(()=>{}, []);
export const getStaticPaths = ()=>{};
export default _withSuperJSONPage(()=>{
    return <></>;
});
