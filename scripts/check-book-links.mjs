import { readFile, readdir, stat } from 'node:fs/promises'
import { resolve, relative, dirname, posix } from 'node:path'
const root=resolve(new URL('..',import.meta.url).pathname), docs=resolve(root,'docs'), dist=resolve(docs,'.vitepress/dist'), failures=[], checked=[]
const exists=async p=>{try{await stat(p);return true}catch{return false}}
async function files(dir,suffix){const out=[];if(!await exists(dir))return out;for(const e of await readdir(dir,{withFileTypes:true})){const p=resolve(dir,e.name);if(e.isDirectory())out.push(...await files(p,suffix));else if(e.isFile()&&p.endsWith(suffix))out.push(p)}return out}
function external(h){return /^(?:https?:|mailto:|tel:|data:|javascript:)/i.test(h)}
function decode(h){try{return decodeURIComponent(h.replace(/&amp;/g,'&').replace(/&quot;/g,'"').replace(/&#39;/g,"'"))}catch{return h}}
function targetFor(h,source){if(!h||external(h))return null;const sourceRoute=source.startsWith(dist)?'/'+relative(dist,source):'/';const base=`https://local${sourceRoute}`;const url=new URL(h,base);let route=url.pathname.replace(/^\/Rein(?:\/|$)/,'/');if(route.endsWith('.md'))route=route.slice(0,-3)+'.html';if(route.endsWith('/'))route+='index.html';else if(!route.includes('.'))route+='.html';const candidate=resolve(dist,'.'+route);return {candidate:candidate.startsWith(dist+'/')?candidate:null,fragment:url.hash?decode(url.hash.slice(1)):undefined}}
function links(html){return [...html.matchAll(/<a\b[^>]*\bhref\s*=\s*(["'])(.*?)\1[^>]*>/gis)].map(m=>m[2])}
function anchors(html){return new Set([...html.matchAll(/\b(?:id|name)\s*=\s*(["'])(.*?)\1/gi)].map(m=>m[2]))}
const cache=new Map()
async function check(h,source){const t=targetFor(h,source);if(!t)return true;checked.push({source:relative(root,source),href:h});if(!t.candidate||!await exists(t.candidate)){failures.push(`${relative(root,source)} -> ${h} (missing target)`);return false}if(t.fragment){const html=cache.get(t.candidate)??await readFile(t.candidate,'utf8');cache.set(t.candidate,html);if(!anchors(html).has(t.fragment)){failures.push(`${relative(root,source)} -> ${h} (missing fragment #${t.fragment})`);return false}}return true}
for(const f of await files(dist,'.html')){const html=await readFile(f,'utf8');cache.set(f,html);for(const h of links(html))await check(h,f)}
const config=resolve(docs,'.vitepress/config.mts');if(await exists(config)){const text=await readFile(config,'utf8');for(const m of text.matchAll(/\blink\s*:\s*['"]([^'"]+)['"]/g))await check(m[1],config)}
const cases=[['knownvalidpage','/chapters/05-rust.html',true],['missingpage','/__checker_missing__.html',false],['validhash','/chapters/05-rust.html#loop-entry',true],['missinghash','/chapters/05-rust.html#__checker_missing__',false],['encodedhash','/chapters/05-rust.html#loop%2Dentry',true],['relativeURL','./05-rust.html',true],['externalURL','https://example.com/x',true]]
const selfTest={};for(const [name,href,expected] of cases){const before=failures.length;const ok=await check(href,resolve(dist,'chapters/06-rust.html'));const added=failures.splice(before);selfTest[name]=(ok===expected)&&added.length===(expected?0:1)}
console.log(JSON.stringify({checked:checked.length,failures,selfTest},null,2));if(!Object.values(selfTest).every(Boolean)||failures.length)process.exitCode=1
