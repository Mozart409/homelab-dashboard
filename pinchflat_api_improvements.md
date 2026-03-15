# Pinchflat API Improvements

Issues discovered while integrating with the Pinchflat API for the homelab dashboard.

## 1. `/api/media/recent_downloads` missing nested `source` object

**Issue:** The OpenAPI spec indicates that `RecentDownloadsResponse` returns `MediaItem` objects which include a nested `source` field (via `$ref`), but the actual API response only includes `source_id` without the nested source object.

**Expected (per spec):**
```json
{
  "data": [
    {
      "id": 714557,
      "title": "Video Title",
      "source_id": 4,
      "source": {
        "id": 4,
        "custom_name": "Channel Name",
        "collection_name": "...",
        ...
      },
      ...
    }
  ]
}
```

**Actual:**
```json
{
  "data": [
    {
      "id": 714557,
      "title": "Video Title",
      "source_id": 4,
      ...
    }
  ]
}
```

**Workaround:** Dashboard fetches `/sources` separately and maintains a cached lookup map of `source_id -> custom_name`.

**Suggested Fix:** Preload the source association in the `recent_downloads` controller action, similar to how `/api/media` does it.

## 2. Consider adding `source` preload to recent_downloads endpoint

The `/api/media?limit=N` endpoint includes the full nested `source` object, but `/api/media/recent_downloads` does not. For dashboard use cases, having the source info inline avoids an extra API call.

**Location to fix:** Likely in the Elixir controller for `Api.MediaController.recent_downloads` - add a `preload: [:source]` or similar to the query.
