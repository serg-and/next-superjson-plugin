import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONInitProps as _withSuperJSONInitProps } from "superjson-next/tools";
function Page({ date }) {
    return <div>{date.getDate()}</div>;
}
Page.getInitialProps = _withSuperJSONInitProps(()=>{
    return {
        date: new Date()
    };
}, []);
export default _withSuperJSONPage(Page);
