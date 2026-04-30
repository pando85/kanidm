
PORTNAME=	kanidm
# DISTVERSION=    1.5.0-dev
# DISTVERSIONPREFIX=	v

DISTVERSION=    g20250102
GH_TAGNAME=     edb8cccc84e9dacd2ac31ea1162dd24c0c454c55
GH_ACCOUNT=     Firstyear

CATEGORIES=	security net databases

LICENSE=        MPL20
LICENSE_FILE=   ${WRKSRC}/LICENSE.md
MAINTAINER=	william@blackhats.net.au
COMMENT=	Simple and secure identity management platform
WWW=		https://github.com/kanidm/kanidm/

USES=		cargo ssl
USE_GITHUB=	yes

ONLY_FOR_ARCHS=	aarch64 amd64

CARGO_ENV=      KANIDM_BUILD_PROFILE=release_freebsd

CARGO_BUILD_ARGS = -p kubidm_tools -p kubidm_unix_int -p nss_kubidm -p pam_kubidm

CARGO_INSTALL=  no

USE_RC_SUBR=	kubidm_unixd kubidm_unixd_tasks

USERS=  _kubidm_unixd
GROUPS= _kubidm_unixd

do-install:
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm ${STAGEDIR}${PREFIX}/bin
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm-unix ${STAGEDIR}${PREFIX}/bin
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm_ssh_authorizedkeys ${STAGEDIR}${PREFIX}/bin
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm_ssh_authorizedkeys_direct ${STAGEDIR}${PREFIX}/bin
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm_unixd ${STAGEDIR}${PREFIX}/libexec
	${INSTALL_PROGRAM} ${WRKDIR}/target/release/kubidm_unixd_tasks ${STAGEDIR}${PREFIX}/libexec
	${INSTALL_LIB} ${WRKDIR}/target/release/libnss_kubidm.so ${STAGEDIR}${PREFIX}/lib/nss_kubidm.so.1
	${INSTALL_LIB} ${WRKDIR}/target/release/libpam_kubidm.so ${STAGEDIR}${PREFIX}/lib
	${MKDIR} ${STAGEDIR}${PREFIX}/etc
	${MKDIR} ${STAGEDIR}${PREFIX}/etc/kubidm
	${MKDIR} ${STAGEDIR}/var/run/kubidm-unixd
	${MKDIR} ${STAGEDIR}/var/lib/kubidm-unixd
	${MKDIR} ${STAGEDIR}/var/cache/kubidm-unixd

.include <bsd.port.mk>
