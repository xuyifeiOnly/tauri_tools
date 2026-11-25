export interface Mapping { id:string; name:string; localPath:string; route:string; enabled:boolean; allowLan:boolean; username:string; passwordHash:string }
export interface Config {host:string;sshPort:number;user:string;localPort:number;remotePort:number;publicOrigin:string;basePath:string;keyId:string;mappings:Mapping[]}
export interface KeyInfo {id:string;publicKey:string;fingerprint:string;privateKeyPath:string;publicKeyPath:string}
export interface RuntimeStatus {local:boolean;tunnel:string;public:string;publicCheckedAt:number|null;logs:string[]}
export const defaults:Config={host:'',sshPort:22,user:'image-tunnel',localPort:20005,remotePort:18790,publicOrigin:'',basePath:'/my_mac',keyId:'',mappings:[]}
