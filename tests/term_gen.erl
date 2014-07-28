#!/usr/bin/env escript
-mode(compile).
%-module(term_gen).
-export([main/1]).

main([]) ->
    DataDir = filename:join("tests", "data"),
    ok = filelib:ensure_dir(DataDir),
    W = fun(Name, Term) ->
                ok = file:write_file(
                       filename:join(DataDir, Name ++ ".bin"),
                       term_to_binary(Term))
        end,
    lists:foreach(fun({N, T}) -> W(N, T) end, primitive_terms()),
    EmptyContainers = lists:map(fun({_, Gen}) -> Gen([]) end, container_terms()),
    lists:foreach(fun({Name, Gen}) ->
                          Term = Gen(EmptyContainers
                                     ++ [T || {_, T} <- primitive_terms()]),
                          W(Name, Term)
                  end, container_terms()).

primitive_terms() ->
    [{"SmallInt-min", 1},
     {"SmallInt-max", 255},
     {"Int-min", 256},
     {"Int-max", 2147483647},
     {"Int-neg-max", -2147483647},
     {"Int-neg-min", -1},
     {"Float-zero", 0.0},
     {"Float-neg", -11111111111.1},
     {"Float-pos", 11111111111.1},
     {"SmallBig-min", 2147483648},
     {"SmallBig-neg-min", -2147483649},
     {"LargeBig", (fun() ->
                           N = trunc(math:pow(255, 128)),
                           N * N
                   end)()},
     %% {"LargeBig 1", begin N = trunc(math:pow(128, 128)), N * N * N * N * N ... end},
     %% {"LargeBig 2", -2147483649},
     {"Reference", make_ref()},
     %% {"SmallAtom", some_atom}, % don't supported by term_to_binary
     {"Atom", list_to_atom(string:copies("a", 255))},
     {"Pid", self()},
     {"Port", (fun() ->
                       try lists:last(erlang:ports())
                       catch _:_ ->
                               erlang:open_port({spawn, "/bin/true"}, [])
                       end
               end)()},
     {"Nil", []},
     {"Binary", <<0, 1, 2, 33, 44, 55, 66, 77, 88, 99, 110, 220, 230, 240, 255>>},
     {"BitBinary", <<0, 7:1>>},
     {"Fun", fun(A) -> A end},
     {"Export", fun erlang:term_to_binary/1}].

container_terms() ->
    [{"List", fun(Terms) -> Terms end},
     {"Tuple", fun(Terms) -> list_to_tuple(Terms) end}, %small & large tuples
     {"Map", fun(Terms) -> maps:from_list([{T, T} || T <- Terms]) end},
     {"String", fun(_) -> lists:seq(0, 255) end}].
