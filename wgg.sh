PEERFILE=/etc/wireguard/peers
WGCOMMAND=$(which wg)

TPUT=$(which tput 2>/dev/null)
[ -z $TPUT ] && TPUT=tputf

# Make sure peer file exists
if [[ ! -f "$PEERFILE" ]]; then
  touch "$PEERFILE" 2>/dev/null

  if [[ "$?" != "0" ]]; then
    echo "Peer file $PEERFILE is not accesible by your user"

    exit 0
  fi
fi

function tputf() {

  case "$1" in
    "setaf")
        col=$2
        case "$col" in
            ''[0-9])
                printf "\E[3"$col"m"
                ;;
            *)
                ;;
        esac
        ;;

    "setab")
        col=$2
        case "$col" in
            ''[0-9])
                printf "\E[4"$col"m"
                ;;
            *)
                ;;
        esac
        ;;

    "bold")
        printf '\E[1m'
        ;;

    "sgr0")
        printf '\E(B\E[m'
        ;;
    *)
        ;;

  esac
}

function updatePeerFile() {
  local NEWPEERS=()

  # Loop config, extract peers, check peers file, add if not present
  IFS=$'\r'
  while read LINE ; do
    # Check if its a peer line
    if [[ $LINE == *"peer"* ]]; then
      # Isolate peer public key, cut peer: (hardcoded)
      PEERPK=$(printf '%s' "$LINE" | cut -c7-)

      # See if we can find peer in peers file
      PEERCOUNT=$(grep $PEERPK "$PEERFILE" | wc -l)

      if [[ $PEERCOUNT -eq 0 ]]; then
        # Peer not found in peers file, add for later processing
        NEWPEERS+=("$PEERPK")
      fi
    fi
  done <<< $("$WGCOMMAND")

  for PEERPK in "${NEWPEERS[@]}"; do
    echo -n "Enter friendly name for peer "
    "$TPUT" setaf 7; "$TPUT" bold
    echo -n $PEERPK
    "$TPUT" setaf 9; "$TPUT" sgr0
    read -r -p " : " PEERNAME

    if [[ "$PEERNAME" == "" ]]; then
      PEERNAME="Unnamed peer"
    fi

    echo "$PEERPK:$PEERNAME" >> "$PEERFILE"
  done
}

function showConfiguration() {
  # Determine if we are using rich (colorful) output or not
  local RICHOUTPUT=1;

  if [[ ! -t 1 ]]; then
    RICHOUTPUT=0
  fi

  curdate=$(date +%s)

  # Run wg through script to preserve color coding
  script --flush --quiet /dev/null --command "$WGCOMMAND show $DEVLIST" | while read LINE ; do
    # Check if its a peer line
    if [[ $LINE == *"peer"* ]]; then
      # Isolate peer public key, cut peer: (incl colors) hardcoded, then cut until first ESC character
      PEERPK=$(printf '%s' "$LINE" | cut -c25- | cut -d $(echo -e '\033') -f1)

      # Output peer line
      echoLine "$LINE" $RICHOUTPUT 1

      # See if we can find peer in peers file
      PEER=$(grep $PEERPK "$PEERFILE" | cut -d ':' -f2)

      # Get latest handshake and idle time for this peer
      last_handshake=$("$WGCOMMAND" show all latest-handshakes | grep $PEERPK | cut -f3)
      idle_seconds=$((${curdate}-${last_handshake}))

      # Choose color
      if [[ $idle_seconds -lt 150 ]]; then
        color=2 #green
      else if [[ $last_handshake -gt 0 ]]; then
             color=3 #yellow
           else
             color=4 #blue
           fi
      fi

      # If we found a friendly name, print that
      if [[ "$PEER" != "" ]]; then
        # Pretty print friendly name
        echoLine "$(printf '%s' "$("$TPUT" bold)$("$TPUT" setaf 7)  friendly name$("$TPUT" setaf 9)$("$TPUT" sgr0)")" $RICHOUTPUT 0
        echoLine "$(printf '%s%s' ": " "$("$TPUT" bold)$("$TPUT" setab $color)$PEER$("$TPUT" sgr0)")" $RICHOUTPUT 1
      fi
    else
      # Non-peer line, just output, but remember indentation
      if [[ "$LINE" == *"interface"* ]]; then
        echoLine "$LINE" $RICHOUTPUT 1
      else
        echoLine "  $LINE" $RICHOUTPUT 1
      fi
    fi
  done
}

# $1: text, $2 richoutput, $3 print linebreak
function echoLine() {
  # Strip any newline characters
  local OUTPUTLINE=$(printf '%s' "$1" | sed '$ s/\[\r\n]$//')

  # If not rich output, strip ANSI control codes
  if [[ $2 -eq 0 ]]; then
    OUTPUTLINE=$(printf '%s' "$OUTPUTLINE" | sed 's/\x1b\[[0-9]\{0,\}m\{0,1\}\x0f\{0,1\}//g')
  fi

  # Handle newline printing
  if [[ $3 -eq 0 ]]; then
    printf '%s' "$OUTPUTLINE"
  else
    printf '%s\r\n' "$OUTPUTLINE"
  fi
}

# What are we doing?
while getopts ":up:" OPTION; do
  case ${OPTION} in
    u)  updatePeerFile
        exit
        ;;
    p)  PEERPK=${OPTARG}
        PEER=$(grep $PEERPK "$PEERFILE" 2> /dev/null | cut -d ':' -f2)
        [[ "$PEER" != "" ]] && echo "$PEER"
        exit
        ;;
    :)  >&2 echo "Option -${OPTARG} requires an argument."
        exit
        ;;
  esac
done

shift "$(( OPTIND - 1 ))"

if [[ "$1" != "" ]]; then
	DEVLIST=$1
else
	DEVLIST=all
fi

# Show the peer-enriched configuration overview
showConfiguration

exit
