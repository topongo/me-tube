// home_screen.dart
import 'package:MeTube/users_screen.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/svg.dart';
import 'upload.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class HomeScreen extends StatefulWidget {
  @override
  _HomeScreenState createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final List<dynamic> _videos = [];
  late Map<String, dynamic> _games;
  late Map<String, dynamic> _userGames;
  late Set<String> _likedVideos;
  late Map<String, int> _videoLikes;
  late int _totalVideos;
  late ScrollController _scrollController;
  bool _init = false;
  Map<String, dynamic>? _files;
  bool isLoading = false;
  bool hasMore() => _videos.length < _totalVideos;
  final Map<String, bool Function(dynamic)> _filters = {};

  get _filteredVideos => _videos.indexed.where((v) => !_filters.values.any((f) => f(v.$2))).map((v) => v.$1).toList();
  
  Future<void> getFilesAndLikes() async {
    if (_files != null) return;
    final auth = Provider.of<AuthService>(context, listen: false);
    final fResponse = await auth.api("video/file");
    final Map<String, dynamic> files = {for (final f in fResponse) f['_id']: f};
    final likes = await auth.api("like");
    final Set<String> likedVideos = {for (final l in likes) l};
    final Map<String, dynamic> lResponse = await auth.api("video/like");
    final Map<String, int> videoLikes = {for (final e in lResponse.entries) e.key: e.value};
    final Map<String, dynamic> games = {for (final g in await auth.api("game")) g['_id']: g};
    final Map<String, dynamic> userGames = {for (final g in await auth.api("game/user/${auth.username}")) g['_id']: g};
    setState(() {
      _files = files;
      _likedVideos = likedVideos;
      _videoLikes = videoLikes;
      _games = games;
      _userGames = userGames;
    });
  }

  Future<void> loadMore() async {
    if (_init && !hasMore()) return;
    if (isLoading) return;
    setState(() => isLoading = true);
    await getFilesAndLikes();
    if (context.mounted) {
      try {
        final auth = Provider.of<AuthService>(context, listen: false);
        final (videos, headers) = await auth.apiAndHeaders("video", query: "?skip=${_videos.length.toString()}&limit=20");
        final totalCount = int.parse(headers['x-total-count']!);
        setState(() {
          for (final v in videos) {
            v['file'] = _files![v['file']];
            v['likes'] = _videoLikes[v['_id']];
            _videos.add(v);
          }
          _totalVideos = totalCount;
          isLoading = false;
          _init = true;
          // print('page status: hasMore: ${hasMore()}, totalVideos: $_totalVideos, videos: ${_videos.length}');
        });
      } catch (e) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("$e")));
      }
    }
  }

  @override
  void initState() {
    super.initState();
    _scrollController = ScrollController()..addListener(() {
      // print("scrolling: ${_scrollController.position.pixels}");
      final fetchTrigger = .8 * _scrollController.position.maxScrollExtent;
      if (_scrollController.position.pixels > fetchTrigger) {
        // print("triggered: hasMore: ${hasMore()}");
        if (hasMore()) loadMore();
      }
    });
    loadMore();
  }

  void _refresh() { 
    setState(() {
      _init = false;
      _videos.clear();
      _totalVideos = 0;
      _likedVideos.clear();
      _videoLikes.clear();
      _files = null;
    });
    loadMore();
  }

  @override
  void setState(fn) {
    super.setState(fn);
    if (_init && hasMore() && _filteredVideos.toList().length < 20) {
      loadMore();
    }
  }

  @override
  Widget build(BuildContext context) {
    final auth = Provider.of<AuthService>(context, listen: false);
    final List<int> filteredVideos = _filteredVideos;
    return Consumer<AuthService>(
      builder: (context, authService, child) {
        if (authService.isAuthenticated) {
          return child!;
        } else {
          Future.delayed(Duration(milliseconds: 200), () {
            if (context.mounted) {
              Navigator.pushReplacementNamed(context, "/login");
            }
          });
          return Scaffold();
        }
      },
      child: Scaffold(
        appBar: AppBar(
          // title: Image.asset('assets/logo.svg', height: 30),
          title: SvgPicture.asset('assets/logo.svg', height: 30),
          actions: [
            auth.isAdmin! ? IconButton(
              icon: Icon(Icons.people),
              onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (context) => UsersScreen())),
            ) : Container(),
            IconButton(
              icon: Icon(Icons.logout),
              onPressed: () async { await auth.logout(); },
            ),
            IconButton(
              icon: Icon(Icons.refresh),
              onPressed: _refresh,
            )
          ],
        ),
        floatingActionButton: FloatingActionButton(
          onPressed: () async {
            final List<dynamic> result = await Navigator.push(context, MaterialPageRoute(builder: (context) => UploadScreen()));
            if (result.isNotEmpty) {
              _refresh();
            }
          },
          child: Icon(Icons.upload),
        ),
        body: !_init ? const Center(child: Column(mainAxisSize: MainAxisSize.min, children: [CircularProgressIndicator()])) : ListView.builder(
              controller: _scrollController,
              itemCount: (hasMore() ? 2 : 1) + filteredVideos.length,
              itemBuilder: (context, index) {
                if (index == 0) {
                  return Center(child: Wrap(
                    crossAxisAlignment: WrapCrossAlignment.center,
                    spacing: 5,
                    children: [
                      for (final g in _games.entries)
                        FilterChip(
                          label: Text(g.value["name"]),
                          onSelected: (selected) {
                            setState(() {
                              if (selected) {
                                _filters.remove(g.key);
                              } else {
                                _filters[g.key] = (v) => v['game'] == g.key;
                              }
                            });
                          },
                          selected: !_filters.containsKey(g.key),
                        )
                    ]
                  ));
                } else {
                  index -= 1;
                }
                if (index == filteredVideos.length) {
                  if(hasMore()) {
                    return Center(child: Padding(
                      padding: EdgeInsets.all(15),
                      child: CircularProgressIndicator(),
                    ));
                  } else {
                    return Container();
                  }
                }
                final v = _videos[filteredVideos[index]];
                return VideoTile(
                  video: v, 
                  game: _games[v['game']]['name'], 
                  userGames: _userGames, 
                  likes: _likedVideos, 
                  notifyParent: () => setState(() {}),
                  deleteSelf: () => setState(() { _videos.removeAt(filteredVideos[index]); _totalVideos -= 1; }),
                );
              },
            // )
          // ],
        ),
      )
    );
  }
}

class VideoTile extends StatefulWidget {
  final Map<String, dynamic> video;
  final String game;
  final Map<String, dynamic> userGames;
  final Set<String> likes;
  final Function() notifyParent;
  final Function() deleteSelf;

  VideoTile({
    required this.video, 
    required this.notifyParent, 
    required this.likes, 
    required this.game,
    required this.userGames,
    required this.deleteSelf,
    super.key
  });

  @override
  _VideoTileState createState() => _VideoTileState();
}

class _VideoTileState extends State<VideoTile> {
  @override
  Widget build(BuildContext context) {
    // print("constructing video card for ${widget.video['_id']}");
    return GestureDetector(
      onTap: () {
        Navigator.pushNamed(context, "/watch/${widget.video["_id"]}");
      },
      child: ListTile(
        leading: CachedNetworkImage(
          imageUrl: "${AuthService.baseUrl}/video/${widget.video['file']['_id']}/thumb",
          placeholder: (context, url) => CircularProgressIndicator(),
          errorWidget: (context, url, error) => Image.asset("assets/thumb.png"),
        ),
        title: Text(widget.video['name'] ?? widget.video['_id']),
        // subtitle: Text(widget.video['file']['video_codec']),
        subtitle: Text("${widget.game} - ${widget.video['owner']}"),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                IconButton(
                  icon: Icon(widget.likes.contains(widget.video['_id']) ? Icons.favorite : Icons.favorite_border),
                  onPressed: () async {
                    final auth = Provider.of<AuthService>(context, listen: false);
                    await auth.api("video/${widget.video['_id']}/like", method: widget.likes.contains(widget.video['_id']) ? "DELETE" : "POST");
                    setState(() {
                      if (widget.likes.contains(widget.video['_id'])) {
                        widget.likes.remove(widget.video['_id']);
                        widget.video['likes'] -= 1;
                      } else {
                        widget.likes.add(widget.video['_id']);
                        widget.video['likes'] += 1;
                      }
                    });
                  },
                ),
                Text(widget.video['likes'].toString())
              ]
            ), 
            IconButton(
              icon: Icon(Icons.more_vert),
              onPressed: () {
                showModalBottomSheet(context: context, builder: (context) {
                  return VideoCard(
                    video: widget.video, 
                    userGames: widget.userGames, 
                    notifyParent: widget.notifyParent, 
                    deleteSelf: () { widget.deleteSelf(); Navigator.pop(context); Navigator.pop(context); },
                    game: widget.game,
                  );
                });
              },
            ),
          ]
        ),
      )
    );
  }
}

class VideoCard extends StatefulWidget {
  final Map<String, dynamic> video;
  final Map<String, dynamic> userGames;
  final String game;
  final Function() notifyParent;
  final Function() deleteSelf;

  VideoCard({required this.video, required this.userGames, required this.notifyParent, required this.deleteSelf, required this.game, super.key});

  @override
  _VideoCardState createState() => _VideoCardState();
}

class _VideoCardState extends State<VideoCard> {
  Future<void> _updateVideo(String key, dynamic value, BuildContext context, {bool? dontPop}) async {
    try {
      final auth = Provider.of<AuthService>(context, listen: false);
      await auth.api("video/${widget.video['_id']}", method: "POST", data: {key: value});
      if (dontPop != true && context.mounted) {
        widget.notifyParent();
        Navigator.pop(context);
      }
      setState(() {
        widget.video[key] = value;
      });
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("$e")));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        ListTile(
          leading: Icon(Icons.play_arrow),
          title: Text(widget.video["name"] ?? widget.video["_id"], style: Theme.of(context).textTheme.headlineMedium),
          subtitle: Text("${widget.game} - ${widget.video["owner"]}"),
          trailing: Icon(widget.video["public"] ? Icons.public : Icons.lock),
        ),
        Divider(),
        ActionButton(
          icon: !widget.video['public'] ? Icons.public : Icons.lock,
          text: widget.video['public'] ? 'Make Private' : 'Make Public',
          onPressed: () async => await _updateVideo('public', !widget.video['public'], context, dontPop: true),
        ),
        !widget.video['public'] ? Container() : ActionButton(
          icon: Icons.share,
          text: "Copy Share Link",
          onPressed: () async {
            await Clipboard.setData(ClipboardData(text: "https://metube.prabo.org/share/${widget.video['_id']}"));
            if (context.mounted) {
              ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Link copied to clipboard")));
            }
          }
        ),
        ActionButton(
          icon: Icons.edit,
          text: "Edit Name",
          onPressed: () {
            showDialog(
              context: context,
              builder: (context) {
                final nameController = TextEditingController(text: widget.video['name']);
                return AlertDialog(
                  title: Text('Edit Name'),
                  content: TextField(
                    controller: nameController,
                    decoration: InputDecoration(labelText: 'Name'),
                    onEditingComplete: () async => await _updateVideo('name', nameController.text, context),
                    autofocus: true,
                  ),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.pop(context),
                      child: Text('Cancel'),
                    ),
                    TextButton(
                      onPressed: () async => _updateVideo('name', nameController.text, context),
                      child: Text('Save'),
                    ),
                  ],
                );
              },
            );
          },
        ),
        ActionButton(
          icon: Icons.sports_esports,
          text: "Set Game",
          onPressed: () {
            showDialog(
              context: context,
              builder: (context) {
                String game = widget.video['game'];
                return AlertDialog(
                  title: Text('Set Game'),
                  content: GameSelector(
                    game: game,
                    games: widget.userGames,
                    onChanged: (value) => game = value,
                  ),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.pop(context),
                      child: Text('Cancel'),
                    ),
                    TextButton(
                      onPressed: () async => _updateVideo('game', game, context),
                      child: Text('Save'),
                    ),
                  ],
                );
              },
            );
          },
        ),
        ActionButton(
          text: "Delete Video",
          icon: Icons.delete,
          onPressed: () {
            showDialog(
              context: context,
              builder: (context) {
                return AlertDialog(
                  title: Text('Delete Video'),
                  content: Text('Are you sure you want to delete this video?'),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.pop(context),
                      child: Text('Cancel'),
                    ),
                    TextButton(
                      onPressed: () async {
                        final auth = Provider.of<AuthService>(context, listen: false);
                        await auth.api("video/${widget.video['_id']}", method: "DELETE");
                        if (context.mounted) {
                          widget.notifyParent();
                          widget.deleteSelf();
                        }
                      },
                      child: Text('Delete'),
                    ),
                  ],
                );
              },
            );
          }
        ),
        Divider(),
        Text("Created: ${DateTime.parse(widget.video['added']).toLocal()}"),
      ],
    );
  }
}

class ActionButton extends StatelessWidget {
  final IconData icon;
  final String text;
  final Function() onPressed;

  ActionButton({required this.icon, required this.text, required this.onPressed, super.key});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon),
      title: Text(text),
      onTap: onPressed,
    );
  }
}

class GameSelector extends StatefulWidget {
  final Map<String, dynamic> games;
  String game;
  final Function(String) onChanged;

  GameSelector({required this.games, required this.onChanged, required this.game, super.key});

  @override
  _GameSelectorState createState() => _GameSelectorState();
}

class _GameSelectorState extends State<GameSelector> {
  @override
  Widget build(BuildContext context) {
    // print(widget.games);
    return DropdownButton<String>(
      value: widget.game,
      items: widget.games.entries.map((e) => DropdownMenuItem(value: e.key, child: Text(e.value['name']))).toList(),
      onChanged: (value) async {
        setState(() {
          widget.game = value!;
          widget.onChanged(value);
        });
      },
    );
  }
}
