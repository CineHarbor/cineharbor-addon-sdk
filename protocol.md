# Stremio addon 协议（CineHarbor 冻结契约）

> CineHarbor 完全跟随 Stremio addon 协议。本文件是实现的冻结参照，逐项对齐官方契约。
> 权威来源：[stremio-addon-sdk](https://github.com/Stremio/stremio-addon-sdk) 与 [protocol.html](https://stremio.github.io/stremio-addon-sdk/protocol.html)。

## 传输

- 纯 HTTP(S) `GET`，响应 `application/json`。
- 一个 addon = 一个可公开访问的 base URL，按要求暴露以下端点；必含 `GET /manifest.json`。

## 资源类型与端点

| 资源 | 清单内标识 | 端点 |
| --- | --- | --- |
| 清单 | — | `GET /manifest.json` |
| 目录 | `catalog` | `GET /catalog/<type>/<id>.json`；分页 `GET /catalog/<type>/<id>/skip=<n>.json`；带扩展参数 `GET /catalog/<type>/<id>/<extra>.json`（如 `search=matrix`、`genre=Action`） |
| 元数据 | `meta` | `GET /meta/<type>/<id>.json` |
| 播放流 | `stream` | `GET /stream/<type>/<id>.json`（响应字段为 `streams`） |
| 字幕 | `subtitles` | `GET /subtitles/<type>/<id>/<extra>.json` |

## 内容类型 `types`

`movie`、`series`、`channel`、`tv`（直播落入 `tv`）。

## Manifest（`/manifest.json`）

关键字段：

```jsonc
{
  "id": "com.example.addon",      // 反向域名，唯一
  "version": "1.0.0",             // semver
  "name": "Example Addon",
  "description": "...",
  "resources": ["catalog", "meta", "stream"],   // 至少一项
  "types": ["movie", "series"],
  "catalogs": [                    // 声明 resource=catalog 时给目录列表
    {
      "type": "movie",
      "id": "top",
      "name": "Top Movies",
      "extra": [
        { "name": "search", "isRequired": false },
        { "name": "genre", "options": ["Action", "Comedy"], "isRequired": false }
      ]
    }
  ],
  "idPrefixes": ["tt"]             // 可选：限定 id 前缀减少无效调用
  // "behaviorHints": { ... }      // 可选：宿主行为提示（扩展字段一律放这里）
}
```

其他可选全局字段：`icon`、`logo`、`background`、`idPrefixes`。

## Catalog 响应

```jsonc
{ "metas": [ /* MetaPreview */ ] }
```

## Meta 响应（`/meta/<type>/<id>.json`）

```jsonc
{
  "meta": {
    "id": "tt0133093",
    "type": "movie",
    "name": "The Matrix",
    "genres": ["Action"],
    "poster": "https://…", "background": "https://…", "logo": "https://…",
    "description": "…",
    "releaseInfo": "1999", "year": "1999",
    "director": ["…"], "cast": ["…"],
    "imdbRating": "8.7",
    "links": [], "videos": [], "trailers": [],
    "behaviorHints": { }
  }
}
```

## Stream 响应（`/stream/<type>/<id>.json`）

```jsonc
{
  "streams": [
    {
      "name": "…", "title": "1080p",
      "url": "https://…",            // 直链（Web 客户端用）
      // torrent 类用 infoHash + fileIdx 替代直链：
      "infoHash": "…", "fileIdx": 0,
      "ytId": "…",                    // YouTube 流
      "subtitles": [], "behaviorHints": { }
    }
  ]
}
```

## Subtitles 响应

```jsonc
{ "subtitles": [ { "id": "…", "url": "…", "lang": "zh" } ] }
```

## CineHarbor 实现约束

1. 字段、端点与官方契约**逐项对齐**，不自造字段；自有扩展只进 `behaviorHints`。
2. `cineharbor-addon-protocol` crate 定义这些类型与解析/校验；`cineharbor-addon-sdk` 提供 addon 构建与 host 接线。
3. host 聚合多条 `streams` 时按 `name`/源去重，直链优先，归一化到播放器统一模型。
4. 参考 addon（bangumi / douban / live）实现同一契约，保证可被 Stremio 官方客户端直接加载。
