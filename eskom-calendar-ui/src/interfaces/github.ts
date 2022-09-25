import internal from "stream";

export interface IProvince {
    key: string;
    value: string;
}
export interface IAuthor {
    login: string;
    id: number;
    node_id: string;
    avatar_url: string;
    gravatar_id: string;
    url: string;
    html_url: string;
    followers_url: string;
    following_url: string;
    gists_url: string;
    starred_url: string;
    subscriptions_url: string;
    organizations_url: string;
    repos_url: string;
    events_url: string;
    received_events_url: string;
    type: string;
    site_admin: boolean;
}

export interface Uploader {
    login: string;
    id: number;
    node_id: string;
    avatar_url: string;
    gravatar_id: string;
    url: string;
    html_url: string;
    followers_url: string;
    following_url: string;
    gists_url: string;
    starred_url: string;
    subscriptions_url: string;
    organizations_url: string;
    repos_url: string;
    events_url: string;
    received_events_url: string;
    type: string;
    site_admin: boolean;
}
export interface ISuburbData {
    subName: string;
    blockId: string;
}

export interface IAsset {
    url: string;
    id: number;
    node_id: string;
    name: string;
    label: string;
    uploader: Uploader;
    content_type: string;
    state: string;
    size: number;
    download_count: number;
    created_at: Date;
    updated_at: Date;
    browser_download_url: string;
    province: string;
    town: string;
    block: number;
}

export interface IGitHubRelease {
    url: string;
    assets_url: string;
    upload_url: string;
    html_url: string;
    id: number;
    author: IAuthor;
    node_id: string;
    tag_name: string;
    target_commitish: string;
    name: string;
    draft: boolean;
    prerelease: boolean;
    created_at: Date;
    published_at: Date;
    assets: IAsset[];
    tarball_url: string;
    zipball_url: string;
    body: string;
}

export interface IMachineDataResponse {
    data: IMyMachineData[];
    lastRecord: number;
    totalRecords: number;
}

export interface IMyMachineData {
    area_name: string;
    stage: string;
    start: string;
    finsh: string;
    source: string;
    province: string;
    block: string;
}

export interface IMyMachineDataGroupedResponse{
    data: IMyMachineDataGrouped[];
}
export interface IMyMachineDataGrouped {
    area_name: string;
    province: string;
    block: string;
}