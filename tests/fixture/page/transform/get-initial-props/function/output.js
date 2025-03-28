import { withSuperJSONPage as _withSuperJSONPage } from "superjson-next/tools";
import { withSuperJSONInitProps as _withSuperJSONInitProps } from "superjson-next/tools";
function Page({ stars }) {
    return <div>Next stars: {stars}</div>;
}
Page.getInitialProps = _withSuperJSONInitProps(async (ctx)=>{
    const res = await fetch('https://api.github.com/repos/vercel/next.js');
    const json = await res.json();
    return {
        stars: json.stargazers_count
    };
}, []);
export default _withSuperJSONPage(Page);
