import 'package:json_annotation/json_annotation.dart';

part 'authentication.g.dart';

@JsonSerializable()
class UserForm {
  final String username;
  final String password;

  const UserForm({required this.username, required this.password});
  factory UserForm.fromJson(Map<String, dynamic> json) => _$UserFormFromJson(json);
  Map<String, dynamic> toJson() => _$UserFormToJson(this);
}

@JsonSerializable()
class LoginResponse {
  const LoginResponse(this.accessToken);
  final String accessToken;
}
