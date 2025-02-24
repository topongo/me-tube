import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:provider/provider.dart';
import 'auth.dart';

class VideoScreen extends StatefulWidget {
  final dynamic video;
  final String game;
  bool liked;
  VideoScreen({super.key, required this.video, required this.game, required this.liked});

  @override
  _VideoScreenState createState() => _VideoScreenState();
}

class _VideoScreenState extends State<VideoScreen> {
  Media? media;
  late final player = Player();
  late final controller = VideoController(player);
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    final auth = Provider.of<AuthService>(context, listen: false);
    if (player.platform is libmpvPlayer) {
      // (player.platform as dynamic).setProperty('tls-cert-file', 'assets/cert.pem').then((_) {
        // _loadVideo(auth);
        // setState(() => _loaded = true);
      // });
    } else {
      _loadVideo(auth);
    }
  }

  Future<void> _loadVideo(AuthService auth) {
    return auth.getVideo(widget.video["_id"]).then((m) {
      media = m;
      player.open(media!);
      setState(() => _loaded = true);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: ListTile(
          title: Text(widget.video["name"] ?? widget.video["_id"]),
          subtitle: Text("${widget.game} - ${widget.video["owner"]}")
        ),
        elevation: 0,
        backgroundColor: Colors.transparent,
        actions: [
          IconButton(
            icon: Icon(widget.liked ? Icons.favorite : Icons.favorite_border),
            onPressed: () async {
              final auth = Provider.of<AuthService>(context, listen: false);
              try {
                await auth.api("video/${widget.video["_id"]}/like", method: widget.liked ? "DELETE" : "POST");
                if (context.mounted) {
                  setState(() => widget.liked = !widget.liked);
                }
              } catch (e) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Error while liking video: $e')));
                }
              }
            }
          )
        ]
      ),
      extendBodyBehindAppBar: true,
      extendBody: true,
      body: LayoutBuilder(
        builder: (context, constraints) {
          return !_loaded ? const CircularProgressIndicator() : Video(controller: controller);
        }
      )
    );
  }

  @override
  dispose() {
    player.dispose();
    super.dispose();
  }
}
