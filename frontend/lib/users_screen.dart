import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'auth.dart';

class UsersScreen extends StatefulWidget {
  late Map<String, dynamic> _users = {};
  late Map<String, int> _permissionsTable = {};
  late Map<String, dynamic> _games = {};

  @override
  _UsersScreenState createState() => _UsersScreenState();
}

class _UsersScreenState extends State<UsersScreen> {
  @override
  initState() {
    super.initState();
    _loadUsers();
  }

  Future<void> _loadUsers() async {
    final auth = Provider.of<AuthService>(context, listen: false);
    final users = await auth.api("user");
    final Map<String, dynamic> uMap = {for (final u in users) u['username']: u};
    final permissions = await auth.api("user/permissions");
    final Map<String, int> pMap = {for (final MapEntry<String, dynamic> p in permissions.entries) p.key: p.value};
    final Map<String, dynamic> games = {for (final f in await auth.api("game")) f['_id']: f};
    setState(() {
      widget._users = uMap;
      widget._permissionsTable = pMap;
      widget._games = games;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Users'),
      ),
      body: ListView.builder(
        itemCount: widget._users.length,
        itemBuilder: (context, index) {
          final key = widget._users.keys.elementAt(index);
          return ListTile(
            title: Text(key),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                IconButton(
                  icon: Icon(Icons.sports_esports),
                  onPressed: () {
                    showDialog(
                      context: context,
                      builder: (context) {
                        return AlertDialog(
                          title: Text('Games for $key'),
                          content: FutureBuilder(
                            future: () async {
                              final auth = Provider.of<AuthService>(context, listen: false);
                              final r = <String>{};
                              for (final g in await auth.api("game/user/$key")) {
                                r.add(g['_id']);
                              }
                              return r;
                            }(),
                            builder: (context, snapshot) {
                              if (snapshot.connectionState != ConnectionState.done) {
                                return CircularProgressIndicator();
                              }
                              final games = snapshot.data!;
                              final userGames = <String, dynamic>{};
                              for (final f in widget._games.entries) {
                                userGames[f.key] = {};
                                userGames[f.key]['_id'] = f.value['_id'];
                                userGames[f.key]['name'] = f.value['name'];
                                userGames[f.key]['value'] = games.contains(f.key);
                              }
                              return GamesTable(
                                games: userGames,
                                user: key,
                              );
                            }
                          )
                        );
                      }
                    );
                  }
                ),
                IconButton(
                  icon: Icon(Icons.key),
                  onPressed: () {
                    showDialog(
                      context: context,
                      builder: (context) {
                        final table = PermissionTable(
                          permissions: widget._users[key]['permissions'],
                          permissionsTable: widget._permissionsTable,
                        );
                        return AlertDialog(
                          title: Text('Change permissions for $key'),
                          content: table,
                          actions: [
                            TextButton(
                              onPressed: () {
                                Navigator.of(context).pop();
                              },
                              child: Text('Cancel'),
                            ),
                            TextButton(
                              onPressed: () async {
                                final auth = Provider.of<AuthService>(context, listen: false);
                                final permissions = table.permissions;
                                // print(permissions);
                                await auth.api("user/$key", method: 'PATCH', data: {'permissions': permissions});
                                setState(() {
                                  widget._users[key]['permissions'] = permissions;
                                });
                                Navigator.of(context).pop();
                              },
                              child: Text('Save'),
                            ),
                          ]
                        );
                      }
                    );
                  },
                )
              ]
            ),
          );
        },
      ),
    );
  }
}

class PermissionTable extends StatefulWidget {
  int permissions;
  final Map<String, int> permissionsTable;

  PermissionTable({required this.permissions, required this.permissionsTable});

  @override
  _PermissionTableState createState() => _PermissionTableState();
}

class _PermissionTableState extends State<PermissionTable> {
  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (final p in widget.permissionsTable.entries)
          CheckboxListTile(
            title: Text(p.key),
            value: widget.permissions & p.value != 0,
            onChanged: (b) {
              setState(() => widget.permissions = b! ? widget.permissions | p.value : widget.permissions & ~p.value);
            },
          ),
        Text('Permissions code: ${widget.permissions}'),
      ],
    );
  }
}

class GamesTable extends StatefulWidget {
  final Map<String, dynamic> games;
  final String user;

  GamesTable({required this.games, required this.user});

  @override
  _GamesTableState createState() => _GamesTableState();
}

class _GamesTableState extends State<GamesTable> {
  Future<void> _update(String game, bool value) async {
    final auth = Provider.of<AuthService>(context, listen: false);
    await auth.api("game/$game/${widget.user}", method: value ? 'POST' : 'DELETE');
    setState(() => widget.games[game]['value'] = value);
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (final g in widget.games.entries)
          CheckboxListTile(
            title: Text(g.value['name']),
            value: g.value['value'],
            onChanged: (b) async {
              await _update(g.key, b!);
            },
          ),
      ],
    );
  }
}
