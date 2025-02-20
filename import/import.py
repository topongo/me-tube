#!/usr/bin/env python
import toml
import json
from pymongo import MongoClient
from bson import ObjectId
from pathlib import Path
from argparse import ArgumentParser, FileType

if __name__ == "__main__":
    parser = ArgumentParser()
    parser.add_argument("--from", type=Path, dest="frm", required=True, help="Path to the folder that contains videos to import")
    parser.add_argument("--target", type=Path, required=True, help="Target in which the backend take videos from")
    parser.add_argument("--data", type=FileType('r'), required=True, help="Path to json file dumped from Django")
    parser.add_argument("--rocket-config", type=Path, required=True, help="Path to Rocket.toml file")
    parser.add_argument("--db", type=str, required=True, help="Name of database to import to")
    parser.add_argument("--yes", "-y", action="store_true", help="Skip confirmation")
    parser.add_argument("--link-files", action="store_true", help="Link files from --from to --target")
    parser.add_argument("--copy-files", action="store_true", help="Copy files from --from to --target")
    parser.add_argument("--skip-existing-check", action="store_true", help="Skip check when creating db, checks will be made anyway before copying or linking")
    args = parser.parse_args()

    if args.yes is None or not args.yes:
        if input("WARNING! This will overwrite completely the current database. Are you sure? (yes/no) ") != "yes":
            print("Aborted.")
            exit(0)
    if args.link_files and args.copy_files:
        print("Cannot use both --link-files and --copy-files")
        exit(1)

    rocket_config = toml.load(args.rocket_config)

    mongo_url = rocket_config["default"]["databases"]["metube"]["url"]
    client = MongoClient(mongo_url)
    db = client[args.db]
    
    with args.data as f:
        data = json.load(f)

    users_imp = {}
    uploaded_videos = {}
    converted_videos = {}
    thumbnails = {}
    games_imp = {}
    ignore_models = (
        "moviedb.movie",
        "moviedb.thread",
        "moviedb.episode",
        "moviedb.series",
        "moviedb.file",
        "moviedb.imdbentity",
        "video_share.ffmpegjob",
        "base.token",
        "base.ip",
        "sessions.session",
        "contenttypes.contenttype",
        "auth.permission",
        "admin.logentry",
    )
    models = {
        "base.user": users_imp,
        "video_share.uploadedvideo": uploaded_videos,
        "video_share.game": games_imp,
        "video_share.convertedvideo": converted_videos,
        "video_share.thumbnail": thumbnails,
    }
    for d in data:
        if d["model"] in ignore_models:
            continue
        elif d["model"] in models:
            models[d["model"]][d["pk"]] = d["fields"]
        else:
            print(f"Unknown model: {d['model']}")

    uploaders = set()
    for video in uploaded_videos.values():
        uploaders.add(video["owner"])

    usermap = {
        1: "topongo",
        4: "saffron",
        5: "banana",
    }

    # users = {}
    # for pk, u in users_imp.items():
    #     users[pk] = {
    #         "_id": str(ObjectId()),
    #         "username": usermap[pk],
    #         "password": u["password"],
    #     }


    games = {}
    game_users = []
    for pk, g in games_imp.items():
        gid = str(ObjectId())
        for u in g["users"]:
            if u not in usermap:
                # print(f"Unknown user: {users[u]["username"]}")
                continue
            game_users.append({"user": usermap[u], "game": gid})
        games[pk] = {
            "_id": gid,
            "name": g["name"],
        }
    games[None] = {"_id": str(ObjectId()), "name": "No game"}

    print(json.dumps(games))
    print(json.dumps(game_users))


    def parse_video(v, converted_to=None):
        # if v["converted"] is not None:
        #     print(json.dumps(v))
        #     print(json.dumps(converted_videos[v["converted"]]))
        #     break
        id_ = str(ObjectId())
        file = args.frm / v["file"]
        if not args.skip_existing_check:
            if not file.exists():
                raise FileNotFoundError(file)
        # fsize = file.stat().st_size
        ext = v["file"].split(".")[-1]
        if ext not in ("mp4", "mkv"):
            print(f"Unsupported format: {ext}")
            raise
        if v["codec_audio"] not in ("aac", "mp3"):
            print(f"Unsupported audio codec: {v['codec_audio']}")
            raise
        if v["codec_video"] not in ("h264", "hevc"):
            print(f"Unsupported video codec: {v['codec_video']}")
            raise
        vf = {
            "_id": id_,
            "duration": v["_duration"],
            "audio_codec": v["codec_audio"],
            "video_codec": v["codec_video"],
            "format": ext,
            "converted": converted_to,
            # "size": fsize,
        }
        if "custom_name" not in v:
            vid = None
        else:
            vid = {
                "_id": v["code"],
                "file": id_,
                "name": v["custom_name"],
                "game": games[v["game"] or None]["_id"],
                "public": v["public"],
                "owner": usermap[v["owner"]],
                "added": v["added"] + "Z",
            }
        if "thumbnail" in v and v["thumbnail"] is not None:
            thumb = thumbnails[v["thumbnail"]]["file"]
        else:
            thumb = None
        return id_, vf, v["code"], vid, thumb
        
        

    videos = {}
    video_files = {}
    to_copy = []
    to_link = []
    converted_associated = {}
    fnf = 0
    for pk, v in converted_videos.items():
        print("Processing converted_video", pk)
        try:
            id_vf, vf, code, vid, thumb = parse_video(v)
            # videos[code] = v
            video_files[id_vf] = vf
            lfile = ((args.frm / v["file"]).resolve(), args.target / id_vf)
            lthumb = None if thumb is None else ((args.frm / thumb).resolve(), args.target / "thumbs" / id_vf)
            if args.link_files:
                to_link.append(lfile)
                if thumb is not None:
                    to_link.append(lthumb)
            elif args.copy_files:
                to_copy.append(lfile)
                if thumb is not None:
                    to_copy.append(lthumb)

            converted_associated[pk] = id_vf
        except FileNotFoundError as e:
            print("File not found:", e)
            continue

    for pk, v in uploaded_videos.items():
        print("Processing uploaded_video", pk)
        if v["converted"] is not None:
            res = parse_video(v, converted_associated.get(v["converted"]))
        else:
            res = parse_video(v)
        assert v is not None
        id_vf, vf, code, vid, thumb = res
        video_files[id_vf] = vf
        videos[code] = vid
        lfile = ((args.frm / v["file"]).resolve(), args.target / id_vf)
        lthumb = None if thumb is None else ((args.frm / thumb).resolve(), args.target / "thumbs" / id_vf)
        if args.link_files:
            to_link.append(lfile)
            if thumb is not None:
                to_link.append(lthumb)
        elif args.copy_files:
            to_copy.append(lfile)
            if thumb is not None:
                to_copy.append(lthumb)

    for src, dst in to_link:
        assert src.exists()
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.symlink_to(src)
        print("Linked", src, "to", dst)

    for src, dst in to_copy:
        assert src.exists()
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(src.read_bytes())
        print("Copied", src, "to", dst)

    for i in ("games", "game_users", "video_files", "videos"):
        db[i].drop()

    db["games"].insert_many(games.values())
    db["game_users"].insert_many(game_users)
    db["video_files"].insert_many(video_files.values())
    db["videos"].insert_many(videos.values())


# ok data:
# Ok(Document({"_id": String("dX_IiqGA"), "file": String("67b20b2f55ae10c4b901c251"), "name": String("scrubs.mp4"), "game": String("67abc688443a24f685e7afe6"), "public": Boolean(false), "owner": String("topongo"), "added": String("2025-02-16T15:58:39.513745751Z")}))

# err data:
# Ok(Document({"_id": String("3LfJ00wm3LP"), "file": String("67af99598b53de9a438e1749"), "name": String("piedini"), "game": String("67af99468b53de9a438e1677"), "public": Boolean(false), "owner": String("topongo"), "added": String("2023-11-20T21:24:26.878")})
